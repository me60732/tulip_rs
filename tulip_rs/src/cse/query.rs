//! `Request` → `JitPlan` → `run`/`resume`/`get_states` — the query surface
//! for the JIT executor (planner-owned interface compiled into one fused
//! machine loop, closed tail set inlined as CLIF ops).
//!
//! * [`Request`] builds the recipe list. `add(recipe, None)` mirrors hand
//!   `::indicator(None)` (primary-only: disabled optionals print EMPTY arrays
//!   in position, their sinks are excluded from emission and from
//!   `first_bar`-independent lead clamping); `add(recipe, Some(&flags))` turns
//!   optionals on per `Info::optional_outputs`, and [`Request::add_all`] is the
//!   explicit all-on form.
//! * [`Request::build`] merges (CSE) the interface graphs, bakes leads, and
//!   compiles the plan with Cranelift. Immutable, data-length-independent;
//!   build once per query SHAPE and cache (compile ≈ 0.4ms — the
//!   `prepare_plan`/`jit_compile` split), then run per asset.
//! * [`crate::cse::JitPlan::get_states`] reconstitutes the hand
//!   `IndicatorState`s from the compiled chain state — the one-way export
//!   the DB/FFI paths use, never on the hot path.
//!
//! The engine itself ([`crate::cse::jit`]) documents the §5.3 gating /
//! streaming invariants; the parity tests here pin them against the hand
//! indicators (`::indicator()` + `batch_indicator` streaming).

use crate::cse::jit::JitPlan;
use crate::cse::{Node, Recipe, Sink, SinkSrc};
use crate::indicators::ema::State as EmaState;
use crate::types::Warm;
use std::sync::Arc;

/// Ordered list of indicator requests.
#[derive(Default)]
pub struct Request {
    pub(crate) recipes: Vec<Recipe>,
    /// Per-request optional-output flags (parallel to `recipes`).
    pub(crate) opt_flags: Vec<Vec<bool>>,
}

impl Request {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a request (from e.g. `Macd::recipe(&[12.,26.,9.])`) with
    /// hand-`::indicator`-parity optional-output flags — the ONE builder
    /// method, mirroring `indicator(inputs, options, Option<&[bool]>)`.
    /// `None` ⇒ every optional off (primary-only emission, the hand `None`
    /// call); `Some(flags)` ⇒ per declared optional in `Info::optional_outputs`
    /// order, missing tail = off. Disabled optionals print EMPTY arrays in
    /// declared position; enabled arrays and primaries are bit-identical to
    /// the all-on run.
    pub fn add(mut self, recipe: Recipe, optional_outputs: Option<&[bool]>) -> Self {
        let n = recipe.optional_outputs.len();
        let mut flags = vec![false; n];
        if let Some(f) = optional_outputs {
            for i in 0..n.min(f.len()) {
                flags[i] = f[i];
            }
        }
        self.recipes.push(recipe);
        self.opt_flags.push(flags);
        self
    }

    /// Append emitting **every** declared optional output (full `Results`).
    /// Explicitly named so "all-on" is never the silent default — the plain
    /// [`add`] mirrors the hand and defaults to primary-only (`None`).
    pub fn add_all(self, recipe: Recipe) -> Self {
        let n = recipe.optional_outputs.len();
        self.add(recipe, Some(&vec![true; n]))
    }

    /// Partition this request's indicators into **sharing groups**: request
    /// indices that reach a common computed node (a chain / arith — not a raw
    /// price `Input`) group together; disjoint ones split. The engine uses this
    /// to run one fused loop *per group* rather than one monolithic loop for the
    /// whole request (measured: fusing disjoint indicators into one loop gives no
    /// dedup and worse — see `natr`+`adx`). Cheap: merge + a union-find walk, no
    /// codegen. Exposed for measurement/diagnostics; the grouping is applied
    /// internally once the composite-plan path lands.
    pub fn sharing_groups(&self) -> Vec<Vec<usize>> {
        let (arena, mut resolved) = crate::cse::merge::merge(&self.recipes);
        for (r, f) in resolved.iter_mut().zip(&self.opt_flags) {
            r.optional_enabled = f.clone();
        }
        arena.sharing_groups(&resolved)
    }

    /// Merge (CSE) the interface graphs, bake leads, apply the optional
    /// switch, and compile the fused JIT loop. Panics on arena shapes the
    /// emitter does not cover yet (reserved node kinds, unknown tail kinds)
    /// — those recipes are the emitter's next coverage, not a fallback path.
    pub fn build(self) -> JitPlan {
        JitPlan::compile(&self)
    }

    /// E2: compile an ASSET-LANE plan — the arena replicates over
    /// `n_assets` price streams (one query shape, many symbols), so the
    /// per-bar math steps pairs of assets per `f64x2` group and one fused
    /// loop advances every symbol's chains, lines, and signals together.
    /// Feed it via [`JitPlan::run_assets`]/`advance_assets`; all streams
    /// carry the same bar count (hand `indicator_by_assets` contract).
    pub fn build_assets(self, n_assets: usize) -> JitPlan {
        JitPlan::compile_assets(&self, n_assets)
    }

    /// E4: [`Self::build`] through the process-wide plan cache — identical
    /// query shapes compile ONCE (~0.4ms) and are then shared as cheap
    /// `Arc<JitPlan>` handles (~µs lookup). Same bytes ⇒ same plan: the
    /// cache is content-addressed, so `Arc::ptr_eq` identifies a shape.
    pub fn build_cached(self) -> Arc<JitPlan> {
        super::cache::cached_plan(&self, 1)
    }

    /// E4: [`Self::build_assets`] through the plan cache.
    pub fn build_assets_cached(self, n_assets: usize) -> Arc<JitPlan> {
        super::cache::cached_plan(&self, n_assets)
    }

    /// E4: deterministic canonical encoding of EVERYTHING that affects
    /// compilation (Handover §4: "serialize the `Request`" — this is that
    /// key). Recipe order, node fields, sink leads, the optional switch,
    /// restorer PRESENCE (not its address — keeps the fingerprint stable
    /// across process restarts), and the asset-lane count. Two requests
    /// with equal bytes merge to equal arenas: same ids, same plan.
    pub(crate) fn canonical_bytes(&self, n_assets: usize) -> Vec<u8> {
        fn u64b(b: &mut Vec<u8>, v: u64) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        fn sinkb(b: &mut Vec<u8>, s: &Sink) {
            let SinkSrc::Interface(id) = s.src;
            u64b(b, id as u64);
            u64b(b, s.lead as u64);
        }
        let mut b: Vec<u8> = Vec::new();
        u64b(&mut b, n_assets as u64);
        u64b(&mut b, self.recipes.len() as u64);
        for (r, flags) in self.recipes.iter().zip(self.opt_flags.iter()) {
            u64b(&mut b, r.kind.len() as u64);
            b.extend_from_slice(r.kind.as_bytes());
            u64b(&mut b, r.options.len() as u64);
            for o in &r.options {
                u64b(&mut b, o.to_bits());
            }
            u64b(&mut b, r.nodes.len() as u64);
            for node in &r.nodes {
                let (tag, x, y, z): (u8, u64, u64, u64) = match *node {
                    Node::Input { stream } => (1, stream as u64, 0, 0),
                    Node::Ema { src, period } => (2, src as u64, period as u64, 0),
                    Node::Sub { a, b } => (3, a as u64, b as u64, 0),
                    Node::Add { a, b } => (19, a as u64, b as u64, 0),
                    Node::Abs { src } => (20, src as u64, 0, 0),
                    Node::Lag { src, n } => (21, src as u64, n as u64, 0),
                    Node::Mul { a, b } => (4, a as u64, b as u64, 0),
                    Node::Div { a, b } => (5, a as u64, b as u64, 0),
                    Node::Max { a, b } => (6, a as u64, b as u64, 0),
                    Node::WhereGe { a, b, t } => (22, a as u64, b as u64, t as u64),
                    Node::Const { bits } => (7, bits, 0, 0),
                    Node::HighPass { input, period } => (8, input as u64, period as u64, 0),
                    Node::SuperSmoother { src, period } => (9, src as u64, period as u64, 0),
                    Node::HtInPhase { src } => (10, src as u64, 0, 0),
                    Node::HtQuadrature { src } => (11, src as u64, 0, 0),
                    Node::Sma { src, period } => (12, src as u64, period as u64, 0),
                    Node::Stddev { src, period } => (13, src as u64, period as u64, 0),
                    // E9: `period` and `look_back` are INDEPENDENT facts — a
                    // `(P, P−1)` window and a `(P, P)` window are different
                    // nodes — so both enter the key (two u32s packed per slot).
                    Node::RollMax {
                        src,
                        period,
                        look_back,
                    } => (
                        24,
                        src as u64,
                        (period as u64) | ((look_back as u64) << 32),
                        0,
                    ),
                    Node::RollMin {
                        src,
                        period,
                        look_back,
                    } => (
                        25,
                        src as u64,
                        (period as u64) | ((look_back as u64) << 32),
                        0,
                    ),
                    Node::Trail { src } => (26, src as u64, 0, 0),
                    Node::Fma { a, b, c } => (14, a as u64, b as u64, c as u64),
                    Node::Sqrt { src } => (15, src as u64, 0, 0),
                    Node::Wma { src, period } => (16, src as u64, period as u64, 0),
                    Node::Hma { src, period } => (17, src as u64, period as u64, 0),
                    Node::Wilders {
                        src,
                        period,
                        composite,
                        feed,
                    } => (
                        18,
                        src as u64,
                        period as u64,
                        (composite as u64) | ((feed as u64) << 1),
                    ),
                    // Five identity fields into three u64 slots: arena ids and
                    // `period` are u32, so two fit per word; `step` is the raw
                    // f64 bit pattern. Injective (plus the recipe `options`
                    // already hashed separately above).
                    Node::SuperTrend {
                        atr,
                        med,
                        close,
                        period,
                        step_bits,
                    } => (
                        23,
                        (atr as u64) | ((med as u64) << 32),
                        (close as u64) | ((period as u64) << 32),
                        step_bits,
                    ),
                };
                b.push(tag);
                u64b(&mut b, x);
                u64b(&mut b, y);
                u64b(&mut b, z);
            }
            u64b(&mut b, r.tail_inputs.len() as u64);
            for &t in &r.tail_inputs {
                u64b(&mut b, t as u64);
            }
            b.push(r.restorer.is_some() as u8);
            u64b(&mut b, r.outputs.len() as u64);
            for o in &r.outputs {
                sinkb(&mut b, o);
            }
            u64b(&mut b, r.optional_outputs.len() as u64);
            for o in &r.optional_outputs {
                sinkb(&mut b, o);
            }
            u64b(&mut b, flags.len() as u64);
            for &f in flags {
                b.push(f as u8);
            }
        }
        b
    }
}

/// Outputs for every request in a run/resume, request-major.
///
/// E2c: slots are `RowRef` VIEWS over one shared buffer per unique output
/// node — mirror lines (three requests' `short_ema`, say) alias the same
/// row the engine wrote once, and ±1 publish-lead differences are windows
/// (an offset), never a copy. Deref makes every existing call site
/// (`res.outputs[r][o].len()`, `[..]`, `&*` → `&[f64]`) work unchanged;
/// code that needs to OWN a row calls `to_vec()`.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// `outputs[request][output]` — primary, in `Info::outputs` order.
    pub outputs: Vec<Vec<RowRef>>,
    /// `optional[request][output]` — `Info::optional_outputs` order; EMPTY
    /// for sinks disabled via [`Request::add`].
    pub optional: Vec<Vec<RowRef>>,
}

// `Results` is `Send + Sync`, so a consumer may fan symbols across a thread
// pool and collect the rows off the workers (review §3.3 / §2.7). Pinned at
// compile time the way `JitPlan` pins itself (jit.rs), so an accidental
// revert to `Rc` fails the build rather than the scanner.
const _: fn() = || {
    fn assert<T: Send + Sync>() {}
    assert::<Results>();
    assert::<RowRef>();
};

/// One output row as written by the plan: a window into the chunk's shared
/// output ARENA (see [`Results`]). `advance` allocates ONE `Arc<[f64]>`
/// per chunk and every store writes its slice of it; each request slot
/// reads `arena[start..start + len]` — mirrors alias, ±1 publish leads are
/// offsets, nothing is copied, and the `Results` clone cost of a 5000-bar
/// dashboard is one refcount bump per row. `Arc` (not `Rc`) so `Results` is
/// `Send` — a consumer may fan symbols across threads and collect the rows
/// (review §3.3 / §2.7).
#[derive(Clone)]
pub struct RowRef {
    arena: Arc<[f64]>,
    start: usize,
    len: usize,
}

impl RowRef {
    pub(crate) fn window(arena: &Arc<[f64]>, start: usize, len: usize) -> Self {
        // a slot whose chunk ENDED before its publish lead projects past a
        // zero-row window (lead_off > rows); clamp to a valid empty view
        let start = start.min(arena.len());
        let len = len.min(arena.len() - start);
        Self {
            arena: Arc::clone(arena),
            start,
            len,
        }
    }
    pub fn as_slice(&self) -> &[f64] {
        &self.arena[self.start..self.start + self.len]
    }
    /// Own a copy (streaming-concat in tests; DB rows that must outlive
    /// the `Results`).
    pub fn to_vec(&self) -> Vec<f64> {
        self.as_slice().to_vec()
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Do two slots read inside the SAME chunk arena? (E2c aliasing.)
    pub fn shares_data_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.arena, &other.arena)
    }
}

impl std::ops::Deref for RowRef {
    type Target = [f64];
    fn deref(&self) -> &[f64] {
        self.as_slice()
    }
}

impl std::fmt::Debug for RowRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl Results {
    pub fn request(&self, i: usize) -> RequestResults<'_> {
        RequestResults {
            outputs: &self.outputs[i],
            optional: &self.optional[i],
        }
    }
    /// Number of request slots in this `Results` (= plans' requests).
    pub fn request_count(&self) -> usize {
        self.outputs.len()
    }
}

/// A single request's outputs within a [`Results`].
pub struct RequestResults<'a> {
    outputs: &'a [RowRef],
    optional: &'a [RowRef],
}

impl<'a> RequestResults<'a> {
    pub fn output(&self, i: usize) -> &'a [f64] {
        self.outputs[i].as_slice()
    }
    pub fn optional(&self, i: usize) -> &'a [f64] {
        self.optional[i].as_slice()
    }
    /// Primary output-row count for this request (≤ `Info::outputs`).
    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }
}
// (the returns above coerce `&RowRef` → `&[f64]` through Deref — the
//  accessor surface is byte-for-byte the pre-E2c API)

/// Concrete hand `IndicatorState`s for DB/FFI/legacy streaming, exported
/// one-way (never fed back into `resume` — independent timelines).
pub enum IndicatorStateBox {
    /// An `Ema` mirror request: exactly the arena node's own interface state.
    Ema(EmaState<Warm>),
    Apo(crate::indicators::apo::State<Warm>),
    Macd(crate::indicators::macd::State<Warm>),
    Ppo(crate::indicators::ppo::State<Warm>),
    /// E3 rolling recurrences — the hand `State<Warm>` (the raw sums +
    /// aux lines). The price window the hand `IndicatorState` also holds is
    /// NOT here: it is the engine's host-side `JitState.window` (the anchor
    /// stitch), which the caller serializes alongside these words.
    Sma(crate::indicators::sma::State<Warm>),
    Stddev(crate::indicators::stddev::State<Warm>),
    Wma(crate::indicators::wma::State<Warm>),
    Wilders(crate::indicators::wilders::State<Warm>),
    Hma(crate::indicators::hma::State<Warm>),
    /// E9 trail-rescan rolls — the hand `{max, trail}` / `{min, trail}`.
    /// The rescan window is the host `JitState.window` like a rolling sum's
    /// price tail, so it is NOT in the box.
    Max(crate::indicators::max::State<Warm>),
    Min(crate::indicators::min::State<Warm>),
    /// E9 M1 — the trail consumers: `aroon` (`{min_state, max_state,
    /// multiplier}`) and `aroonosc` (which wraps the same aroon state).
    Aroon(crate::indicators::aroon::State<Warm>),
    AroonOsc(crate::indicators::aroonosc::State<Warm>),
    /// E9 M2 — same-window consumers: `willr` and `donchianchannel` both carry
    /// exactly `{min_state, max_state}`, so they restore the same way and (when
    /// their windows match) read the SAME two roll chains.
    Willr(crate::indicators::willr::State<Warm>),
    DonchianChannel(crate::indicators::donchianchannel::State<Warm>),
    /// E9 M4 — the densest nesting case: three canonical windows per column, so
    /// the state is six `{value, trail}` pairs with no derived constants.
    Ichimoku(crate::indicators::ichimoku::State<Warm>),
    Other,
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::Planable;
    use crate::indicator_types::Indicator;
    use crate::indicators::apo::Apo;
    use crate::indicators::ema::Ema;
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;

    const APO: [f64; 2] = [12.0, 26.0];
    const MACD: [f64; 3] = [12.0, 26.0, 9.0];
    const PPO: [f64; 2] = [12.0, 26.0];

    fn series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
            .collect()
    }

    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(
                a[i].to_bits(),
                b[i].to_bits(),
                "{what}: mismatch at {i}: {:?} vs {:?}",
                a[i],
                b[i]
            );
        }
    }

    fn classic_request() -> Request {
        Request::new()
            .add_all(Apo::recipe(&APO))
            .add_all(Ppo::recipe(&PPO))
            .add_all(Macd::recipe(&MACD))
    }

    /// THE parity gate: one `run` of the classic merged query == the three
    /// hand `::indicator()` calls, bit-for-bit, primaries AND every optional
    /// line (with flags on), on a long series.
    #[test]
    fn query_matches_hand_indicators() {
        let close = series(5000);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = classic_request().build();
        let (res, _) = plan.run(&s);

        let flags = Some(&[true, true][..]);
        let apo = Apo::indicator(&s, &APO, flags).unwrap().0;
        let ppo = Ppo::indicator(&s, &PPO, flags).unwrap().0;
        let macd = Macd::indicator(&s, &MACD, flags).unwrap().0;

        // apo: [line] + optional [short, long]
        cmp_exact(res.request(0).output(0), &apo[0], "apo_line");
        cmp_exact(res.request(0).optional(0), &apo[1], "apo_short_opt");
        cmp_exact(res.request(0).optional(1), &apo[2], "apo_long_opt");
        // ppo: [ppo] + optional [short, long] — both raw EMA mirrors now
        // (the hand clamp guards only the divisor)
        cmp_exact(res.request(1).output(0), &ppo[0], "ppo_line");
        cmp_exact(res.request(1).optional(0), &ppo[1], "ppo_short_opt");
        cmp_exact(res.request(1).optional(1), &ppo[2], "ppo_long_opt");
        // macd: [line, signal, hist] + optional [short, long]
        cmp_exact(res.request(2).output(0), &macd[0], "macd_line");
        cmp_exact(res.request(2).output(1), &macd[1], "macd_signal");
        cmp_exact(res.request(2).output(2), &macd[2], "macd_hist");
        cmp_exact(res.request(2).optional(0), &macd[3], "macd_short_opt");
        cmp_exact(res.request(2).optional(1), &macd[4], "macd_long_opt");
    }

    /// Streaming contract: 90/140 splits AND a short first chunk
    /// re-concatenate to the single full run, bit-exact, primary+optional.
    #[test]
    fn query_streaming_matches_full_run() {
        let close = series(500);
        let s_all: [&[f64]; 1] = [close.as_slice()];
        let plan = classic_request().build();
        let (full, _) = plan.run(&s_all);

        // Splits include every phase boundary of this plan's leads
        // (mirrors 12, primaries 25/26, signal 33) — boundary arithmetic is
        // exactly where chunking bugs live.
        for split in [90usize, 140, 20, 33, 25, 12, 13, 26, 27, 34] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (head, qs) = plan.run(&s1);
            let (tail, _) = plan.resume(qs, &s2);
            for req in 0..full.outputs.len() {
                for out in 0..full.outputs[req].len() {
                    let mut c = head.outputs[req][out].to_vec();
                    c.extend_from_slice(&tail.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &full.outputs[req][out],
                        &format!("split{split} primary"),
                    );
                }
                for out in 0..full.optional[req].len() {
                    let mut c = head.optional[req][out].to_vec();
                    c.extend_from_slice(&tail.optional[req][out]);
                    cmp_exact(
                        &c,
                        &full.optional[req][out],
                        &format!("split{split} optional"),
                    );
                }
            }
        }
    }

    /// The optional-output switch: `None` matches hand primary-only calls,
    /// disabled slots are EMPTY in position, enabled/primary values are
    /// bit-identical to the all-on run.
    #[test]
    fn optional_switch_prunes_emission_not_values() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];

        let (all, _) = classic_request().build().run(&s);
        let none_plan = Request::new()
            .add(Apo::recipe(&APO), None)
            .add(Ppo::recipe(&PPO), None)
            .add(Macd::recipe(&MACD), None)
            .build();
        let (rn, _) = none_plan.run(&s);

        // hand parity in primary-only mode
        let apo = Apo::indicator(&s, &APO, None).unwrap().0;
        let ppo = Ppo::indicator(&s, &PPO, None).unwrap().0;
        let macd = Macd::indicator(&s, &MACD, None).unwrap().0;
        cmp_exact(rn.request(0).output(0), &apo[0], "hand_apo");
        cmp_exact(rn.request(1).output(0), &ppo[0], "hand_ppo");
        cmp_exact(rn.request(2).output(0), &macd[0], "hand_macd_line");
        cmp_exact(rn.request(2).output(1), &macd[1], "hand_macd_signal");
        cmp_exact(rn.request(2).output(2), &macd[2], "hand_macd_hist");

        for req in 0..3 {
            for out in 0..rn.optional[req].len() {
                assert!(
                    rn.optional[req][out].is_empty(),
                    "req {req} opt {out} disabled must print EMPTY"
                );
                assert!(!all.optional[req][out].is_empty(), "all-on opt empty");
            }
            for out in 0..all.outputs[req].len() {
                cmp_exact(
                    &rn.outputs[req][out],
                    &all.outputs[req][out],
                    "primary invariance",
                );
            }
        }

        // partial: macd long_ema off only — everything else keeps its all-on bits
        let part = Request::new()
            .add(Apo::recipe(&APO), None)
            .add(Ppo::recipe(&PPO), None)
            .add(Macd::recipe(&MACD), Some(&[true, false]))
            .build();
        let (rp, _) = part.run(&s);
        cmp_exact(
            rp.request(2).optional(0),
            all.request(2).optional(0),
            "macd_short_enabled_eq_all",
        );
        assert!(rp.request(2).optional(1).is_empty(), "macd_long disabled");
    }

    /// `get_states` reconstitutes each hand `State` from its arena chains
    /// through the INDICATOR's own restorer (`Recipe::restorer`) — the
    /// engine assembles no state structs and knows no indicator fields
    /// (shared arena node ⇒ identical chains across the three requests:
    /// one authority per node).
    #[test]
    fn get_states_restores_via_indicator_restorers() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = classic_request().build();
        let (res, qs) = plan.run(&s);
        let states = plan.get_states(&qs);

        let (IndicatorStateBox::Apo(ap), IndicatorStateBox::Ppo(pp), IndicatorStateBox::Macd(mc)) =
            (&states[0], &states[1], &states[2])
        else {
            panic!(
                "hand states not rebuilt: {:?}",
                states
                    .iter()
                    .map(|s| matches!(s, IndicatorStateBox::Other))
                    .collect::<Vec<_>>()
            )
        };
        // interface fields == final reported EMA optional line
        assert_eq!(
            ap.short_ema.ema.to_bits(),
            res.request(0).optional(0).last().unwrap().to_bits(),
            "ema12 field vs published"
        );
        assert_eq!(
            ap.long_ema.ema.to_bits(),
            res.request(0).optional(1).last().unwrap().to_bits(),
            "ema26 field vs published"
        );
        // same arena node, one authority: all three requests see identical state
        assert_eq!(pp.short_ema.ema.to_bits(), ap.short_ema.ema.to_bits());
        assert_eq!(mc.short_ema.ema.to_bits(), ap.short_ema.ema.to_bits());
        assert_eq!(mc.long_ema.ema.to_bits(), ap.long_ema.ema.to_bits());
        // macd's signal is TAIL state — must equal the last published signal line
        assert_eq!(
            mc.signal_state.ema.to_bits(),
            res.request(2).output(1).last().unwrap().to_bits(),
            "tail-private signal vs published"
        );
    }

    /// The dedup showcase: adding `Ema` requests to the classic query grows
    /// the arena by NOTHING — the nodes merge into the chains apo/ppo/macd
    /// already advance. Parity vs hand `Ema::indicator` (values AND length),
    /// one-authority identity with apo's optional mirrors, streaming
    /// concat, and `get_states` exporting the interface node itself.
    #[test]
    fn ema_lines_piggyback_shared_nodes() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = classic_request()
            .add(Ema::recipe(&[12.0]), None)
            .add(Ema::recipe(&[26.0]), None)
            .build();
        let classic_nodes = classic_request().build().arena().nodes.len();
        assert_eq!(
            plan.arena().nodes.len(),
            classic_nodes,
            "ema requests must add ZERO nodes to the classic graph"
        );
        assert_eq!(
            classic_nodes, 11,
            "classic graph: chains, the shared line Sub, the ppo guard chain, signal + hist"
        );
        let (res, qs) = plan.run(&s);

        let e12 = Ema::indicator(&s, &[12.0], None).unwrap().0;
        let e26 = Ema::indicator(&s, &[26.0], None).unwrap().0;
        cmp_exact(res.request(3).output(0), &e12[0], "ema12_line");
        cmp_exact(res.request(4).output(0), &e26[0], "ema26_line");
        // One authority per node: the piggybacked line IS apo's optional
        // mirror, byte for byte (classic uses `.add` ⇒ optionals all on).
        cmp_exact(
            res.request(3).output(0),
            res.request(0).optional(0),
            "ema12_eq_apo_short_opt",
        );
        // The long slot on apo reuses the PRIMARY capacity (hand `Apo` fact,
        // lead = long-1), so it prints one earlier bar; from bar `long` on
        // the same arena node agrees bit-exactly with the standalone line.
        cmp_exact(
            &res.request(0).optional(1)[1..],
            res.request(4).output(0),
            "ema26_eq_apo_long_opt_tail",
        );

        // Streaming contract on the mirror request too.
        let split = 90;
        let s1: [&[f64]; 1] = [&close[..split]];
        let s2: [&[f64]; 1] = [&close[split..]];
        let (h1, qs1) = plan.run(&s1);
        let (h2, _) = plan.resume(qs1, &s2);
        let mut c = h1.outputs[3][0].to_vec();
        c.extend_from_slice(&h2.outputs[3][0]);
        cmp_exact(&c, res.request(3).output(0), "ema12_streamed");

        // `get_states`: a mirror request exports the interface node itself,
        // and it must equal the published last value.
        let states = plan.get_states(&qs);
        let IndicatorStateBox::Ema(st12) = &states[3] else {
            panic!("ema interface state not rebuilt");
        };
        assert_eq!(
            st12.ema.to_bits(),
            res.request(3).output(0).last().unwrap().to_bits(),
            "exported node state vs published"
        );
    }
}
