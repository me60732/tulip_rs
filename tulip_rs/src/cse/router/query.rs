//! `Request` → merged arena → fused kernel — the query surface.
//!
//! * [`Request`] builds the recipe list. `add(recipe, None)` mirrors hand
//!   `::indicator(None)` (primary-only: disabled optionals print EMPTY arrays
//!   in position, their stores are gated by `Values.sink_want` at runtime);
//!   `add(recipe, Some(&flags))` turns optionals on per
//!   `Info::optional_outputs`, and [`Request::add_all`] is the explicit
//!   all-on form.
//! * [`crate::cse::erase_request`] merges (CSE) the interface graphs, bakes
//!   leads and want-bits into `Signature`/`Values` — the dispatch key plus
//!   the runtime bindings for the macro-fused kernels (tier 1, see
//!   `proposals/Macro_Cse_Engine/`), the fallback yardsticks for hand and
//!   interpreter, and the input for the routing decision.
//! * The retired Cranelift JIT (`Request::build` → `JitPlan`) now lives in
//!   the top-level `old_jit/` archive; `get_states`/`resume` parity ports to
//!   the kernel registry as M1 (ROADMAP).
//!
//! The parity tests pin streaming invariants against the hand indicators
//! (integration suite: `tulip_test/tests/cse_kernel_registry_test.rs`).

use crate::cse::{ChainState, Node, Recipe, Sink, SinkSrc};
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

    /// Deterministic canonical encoding of EVERYTHING that affects a plan
    /// (Handover §4: "serialize the `Request`" — this is that key). Recipe
    /// order, node fields, sink leads, and the optional switch. Reserved for
    /// the ROUTER's plan cache (R2): the fused kernels are compile-time
    /// registry entries, so nothing calls this yet — the JIT-era
    /// content-addressed `Arc<JitPlan>` cache retired with the engine.
    #[allow(dead_code)]
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
                    Node::WhereLt { a, b, t } => (30, a as u64, b as u64, t as u64),
                    Node::Const { bits } => (7, bits, 0, 0),
                    Node::HighPass { input, period } => (8, input as u64, period as u64, 0),
                    Node::SuperSmoother { src, period } => (9, src as u64, period as u64, 0),
                    Node::HtInPhase { src } => (10, src as u64, 0, 0),
                    Node::HtQuadrature { src } => (11, src as u64, 0, 0),
                    Node::Sma { src, period } => (12, src as u64, period as u64, 0),
                    // E12: the mean/raw switch changes the emitted value, so it
                    // enters the key alongside the period; E13 adds `skip`, which
                    // moves the fold's start and therefore the accumulator's exact
                    // rounding path (measured: a two-bar-early start changes 60% of
                    // window sums in the last bits — see `Node::RollSum`'s doc).
                    Node::RollSum {
                        src,
                        period,
                        mean,
                        skip,
                        steady_from,
                    } => (
                        27,
                        src as u64,
                        (period as u64) | ((mean as u64) << 32),
                        (skip as u64) | ((steady_from as u64) << 32),
                    ),
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
                    // A copy is not a computation, but it IS a distinct node: two
                    // mirrors over different sources must not intern together.
                    Node::Mirror { src } => (29, src as u64, 0, 0),
                    Node::Fma { a, b, c } => (14, a as u64, b as u64, c as u64),
                    Node::Sqrt { src } => (15, src as u64, 0, 0),
                    // E13: the expanded natural log (tag 28) — a distinct
                    // computation from every other node, so it needs its own key.
                    Node::Ln { src } => (28, src as u64, 0, 0),
                    // E15: general recurrence — coefficients are part of identity.
                    Node::Iir {
                        src,
                        a_bits,
                        b_bits,
                    } => (31, src as u64, a_bits, b_bits),
                    // Clamped recurrence: coefficients bit-exact (full x/y); ids
                    // fold into z (plan-CACHE fingerprint; intern keys on Node).
                    Node::IirClamped {
                        src,
                        a_bits,
                        b_bits,
                        hi_t,
                        hi_v,
                        lo_t,
                        lo_v,
                    } => (
                        32,
                        a_bits,
                        b_bits,
                        (src as u64)
                            ^ ((hi_t as u64) << 7)
                            ^ ((hi_v as u64) << 15)
                            ^ ((lo_t as u64) << 23)
                            ^ ((lo_v as u64) << 31),
                    ),
                    Node::Prev { src } => (33, src as u64, 0, 0),
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
    /// E12 — `rsi`: the two MEAN-mode `Wilders` chains (the hand keeps both sums
    /// in one SIMD state) plus `prev_real`, which the recipe carries as an
    /// `Ema{period: 1}` — α=1 makes that recurrence the identity, so its chain
    /// slot holds the last processed close without any new engine concept.
    Rsi(crate::indicators::rsi::State<Warm>),
    /// E12 — `cmo`: two rolling SUMS over the same `up_down` subgraph `rsi`
    /// smooths, so an `rsi`+`cmo` query shares the movement nodes exactly.
    Cmo(crate::indicators::cmo::State<Warm>),
    Other,
}

/// Rebuild one hand `IndicatorState` from its arena chain states — the
/// signature every indicator's `restore_state` matches.
pub type StateRestorer = fn(Vec<ChainState>) -> IndicatorStateBox;

/// The kind → restorer table. Phase 1 of "recipes as data" removed the
/// `Recipe.restorer` field (the leaf crate cannot type a runtime restorer),
/// so the recipe's identity stays a string and this table resolves it.
/// The entries are EXACTLY the 18 kinds whose recipes set
/// `r.restorer = Some(restore_state)` at the pre-JIT-retirement HEAD
/// (`git grep "r.restorer = Some"`) — a phantom entry here would claim the
/// engine can rebuild a hand state it has no wiring for, changing
/// `get_states` behavior; unknown kinds fall to `None`, mirroring the old
/// absent-field path (caller yields `IndicatorStateBox::Other`).
pub fn restorer_for(kind: &str) -> Option<StateRestorer> {
    match kind {
        "ema" => Some(crate::indicators::ema::restore_state),
        "apo" => Some(crate::indicators::apo::restore_state),
        "macd" => Some(crate::indicators::macd::restore_state),
        "ppo" => Some(crate::indicators::ppo::restore_state),
        "wilders" => Some(crate::indicators::wilders::restore_state),
        "sma" => Some(crate::indicators::sma::restore_state),
        "stddev" => Some(crate::indicators::stddev::restore_state),
        "wma" => Some(crate::indicators::wma::restore_state),
        "hma" => Some(crate::indicators::hma::restore_state),
        "max" => Some(crate::indicators::max::restore_state),
        "min" => Some(crate::indicators::min::restore_state),
        "aroon" => Some(crate::indicators::aroon::restore_state),
        "aroonosc" => Some(crate::indicators::aroonosc::restore_state),
        "willr" => Some(crate::indicators::willr::restore_state),
        "donchianchannel" => Some(crate::indicators::donchianchannel::restore_state),
        "ichimoku" => Some(crate::indicators::ichimoku::restore_state),
        "rsi" => Some(crate::indicators::rsi::restore_state),
        "cmo" => Some(crate::indicators::cmo::restore_state),
        _ => None,
    }
}
