//! `Request` → `CompiledPlan` → `run`/`resume`/`get_states`.
//!
//! The "parameterized query" surface: build + optimize (CSE + leads) ONCE, then
//! bind each asset's series at run time (planner §9.1 / Decision Q).
//!
//! * [`CompiledPlan`] — the prepared query: deduped arena + per-node defined-from
//!   `leads` + each request's sink wiring. Immutable, data-length-independent.
//! * [`QueryState`] — per-asset resumable state and the **authority**: one
//!   [`EmaState`] per arena EMA node, so a shared `ema{26}` advances once per bar
//!   in sync across consumers (streaming keeps the dedup; the
//!   "grab one state, batch it alone" desync is structurally impossible).
//! * [`CompiledPlan::get_states`] — **opt-in, one-way** export of concrete
//!   `IndicatorState`s for DB/FFI/legacy streaming; never on the `run` hot path,
//!   never fed back to resume (independent timelines).
//!
//! The streaming interpreter reproduces `plan::{exec, finalize}` exactly: a node
//! is gated to `NAN` before its `lead`, so consumers read a producer only from
//! the producer's *reported* stream (what makes chained EMAs match the hand code).

use crate::cse::{columnar, fuse, merge, simd, Id, Merged, Recipe, Resolved};
use crate::indicators::ema::State as EmaState;
use crate::types::Warm;

/// Ordered list of indicator requests. Each `add` carries the recipe's `kind`
/// tag so [`CompiledPlan::get_states`] can rebuild the concrete state.
#[derive(Default)]
pub struct Request {
    recipes: Vec<Recipe>,
    kinds: Vec<&'static str>,
    /// Per-request optional-output flags (parallel to `recipes`). `add` =
    /// all-on; `add_opt` mirrors hand `::indicator(optional_outputs)`.
    opt_flags: Vec<Vec<bool>>,
}

impl Request {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a request (from e.g. `Macd::recipe(&[12.,24.,9.])`). Every
    /// declared optional output emits (full `Results`).
    pub fn add(mut self, recipe: Recipe) -> Self {
        let kind = recipe.kind;
        let flags = vec![true; recipe.optional_outputs.len()];
        self.recipes.push(recipe);
        self.kinds.push(kind);
        self.opt_flags.push(flags);
        self
    }

    /// Append with hand-`::indicator`-parity optional-output flags: `None` ⇒
    /// every optional OFF (primary-only emission, the hand `None` call);
    /// `Some(flags)` ⇒ per declared optional in `Info::optional_outputs`
    /// order, missing tail = off. Disabled optionals print EMPTY arrays in
    /// declared position; enabled arrays and primaries are bit-identical to
    /// the `add` run, and disabled sinks are excluded from the plan's
    /// `max_publish_lead` (so a sink that only an optional feeds no longer
    /// stretches the gated prologue).
    pub fn add_opt(mut self, recipe: Recipe, optional_outputs: Option<&[bool]>) -> Self {
        let kind = recipe.kind;
        let n = recipe.optional_outputs.len();
        let mut flags = vec![false; n];
        if let Some(f) = optional_outputs {
            for i in 0..n.min(f.len()) {
                flags[i] = f[i];
            }
        }
        self.recipes.push(recipe);
        self.kinds.push(kind);
        self.opt_flags.push(flags);
        self
    }

    /// Merge (CSE) + bake leads + lower the SIMD executor: the one-time
    /// "compile". [`simd::PackedPlan::lower`] returns `None` when the arena
    /// has no ≥2-lane group — `run`/`resume` then fall back to the fused
    /// scalar interpreter (the bit-exact oracle path).
    pub fn build(self) -> CompiledPlan {
        let (arena, mut resolved) = merge(&self.recipes);
        // Apply the per-request optional-output switches before the engines
        // bake their sinks (each engine reads `optional_enabled`).
        for (r, flags) in resolved.iter_mut().zip(&self.opt_flags) {
            r.optional_enabled = flags.clone();
        }
        let leads = arena.leads();
        // Leads are baked into the fused plan + arena here; they need not be
        // stored on `CompiledPlan` (the interpreter that read them is gone).
        let fused = fuse::FusedPlan::build(&arena, &leads, &resolved);
        // The packed executor only engages when the lane packing is dense
        // enough to repay its overhead (see `PackedPlan::worth_it` for the
        // measured cost model); otherwise run/resume stay on the scalar fused
        // path, which is the bit-exact oracle.
        let packed = simd::PackedPlan::lower(&arena, &leads, &resolved).filter(|p| p.worth_it());
        let columnar = columnar::ColumnarPlan::build(&arena, &leads, &resolved);
        CompiledPlan {
            arena,
            resolved,
            kinds: self.kinds,
            fused,
            packed,
            columnar,
        }
    }
}

/// A prepared, reusable query.
pub struct CompiledPlan {
    arena: Merged,
    resolved: Vec<Resolved>,
    kinds: Vec<&'static str>,
    fused: fuse::FusedPlan,
    /// Lane-packed executor when lowering produces a dense-enough pack
    /// (`PackedPlan::worth_it`); otherwise `None`.
    packed: Option<simd::PackedPlan>,
    /// The columnar executor — the default bulk engine (see `advance`).
    columnar: columnar::ColumnarPlan,
}

/// Chunks shorter than this run on the fused per-bar interpreter: columnar
/// amortizes its per-node loop setup + tile scratch only over longer chunks
/// (measured 2026-09: at 1 bar columnar costs ~5% more than fused — 300 vs
/// 285ns — while at ≥ thousands it wins ~2×). Tuned via
/// `cse/resume_one_bar_query` vs `cse/run_one_asset` (Handover §4).
const COLUMNAR_MIN_CHUNK: usize = 16;

impl CompiledPlan {
    pub fn arena(&self) -> &Merged {
        &self.arena
    }

    /// First pass over an asset's full series → outputs + a `QueryState` to
    /// `resume` from (or drop). Zero per-state cost unless you call `get_states`.
    pub fn run(&self, series: &[&[f64]]) -> (Results, QueryState) {
        let mut state = QueryState {
            ema: vec![None; self.arena.nodes.len()],
            next_bar: 0,
        };
        let results = self.advance(&mut state, series);
        (results, state)
    }

    /// Continue an asset from a prior `QueryState`: shared EMA nodes advance
    /// once; every consumer stays in sync by construction.
    pub fn resume(&self, mut state: QueryState, series: &[&[f64]]) -> (Results, QueryState) {
        let results = self.advance(&mut state, series);
        (results, state)
    }

    /// One chunk on the best engine that applies: columnar for chunks of
    /// [`COLUMNAR_MIN_CHUNK`] bars or more (dispatch once per node per chunk,
    /// autovectorized elementwise loops), the lane-packed executor when its
    /// density gate fired at build time, else the fused per-bar scalar
    /// interpreter. All three are bit-identical — each engine's tests pin it
    /// against the `exec`+`finalize` oracle.
    fn advance(&self, state: &mut QueryState, series: &[&[f64]]) -> Results {
        let new_len = series.iter().map(|s| s.len()).max().unwrap_or(0);
        if new_len >= COLUMNAR_MIN_CHUNK {
            return self.columnar.advance(state, series);
        }
        if let Some(p) = &self.packed {
            let mut ps = p.state_from_query(state);
            let res = p.advance(&mut ps, series);
            *state = p.state_to_query(ps);
            return res;
        }
        self.fused.advance(state, series)
    }

    /// Opt-in, one-way export of concrete states (see module docs). Result order
    /// matches the [`Request`] order; unknown kinds yield
    /// [`IndicatorStateBox::Other`].
    pub fn get_states(&self, state: &QueryState) -> Vec<IndicatorStateBox> {
        self.resolved
            .iter()
            .zip(&self.kinds)
            .map(|(r, &kind)| {
                let emas: Vec<EmaState<Warm>> = r
                    .ema_arena_ids
                    .iter()
                    .filter_map(|&id| state.ema[id as usize].clone())
                    .collect();
                IndicatorStateBox::build(kind, emas)
            })
            .collect()
    }
}

/// Per-asset resumable state: the arena authority.
#[derive(Clone)]
pub struct QueryState {
    /// One EMA state per arena node (`None` for non-EMA / not-yet-seeded nodes).
    pub ema: Vec<Option<EmaState<Warm>>>,
    /// Next absolute bar to process (so `run` then `resume` continue cleanly).
    pub next_bar: Id,
}

/// Outputs for every request in a run/resume, request-major.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// `outputs[request][output]` — primary, in `Info::outputs` order.
    pub outputs: Vec<Vec<Vec<f64>>>,
    /// `optional[request][output]` — in `Info::optional_outputs` order.
    pub optional: Vec<Vec<Vec<f64>>>,
}

impl Results {
    pub fn request(&self, i: usize) -> RequestResults<'_> {
        RequestResults {
            outputs: &self.outputs[i],
            optional: &self.optional[i],
        }
    }
}

/// A single request's outputs within a [`Results`].
pub struct RequestResults<'a> {
    outputs: &'a [Vec<f64>],
    optional: &'a [Vec<f64>],
}

impl<'a> RequestResults<'a> {
    pub fn output(&self, i: usize) -> &'a [f64] {
        &self.outputs[i]
    }
    pub fn optional(&self, i: usize) -> &'a [f64] {
        &self.optional[i]
    }
}

/// Concrete `IndicatorState`s for DB/FFI/legacy streaming, exported one-way.
pub enum IndicatorStateBox {
    Apo(crate::indicators::apo::State<Warm>),
    Macd(crate::indicators::macd::State<Warm>),
    Ppo(crate::indicators::ppo::State<Warm>),
    Other,
}

impl IndicatorStateBox {
    /// Rebuild a concrete state from that request's EMA node-states, in the
    /// order the recipe declared them (== the `State` struct's field order).
    fn build(kind: &str, emas: Vec<EmaState<Warm>>) -> Self {
        use crate::indicators::{apo, macd, ppo};
        match kind {
            "apo" if emas.len() == 2 => {
                let mut it = emas.into_iter();
                IndicatorStateBox::Apo(apo::State {
                    short_ema: it.next().unwrap(),
                    long_ema: it.next().unwrap(),
                })
            }
            "ppo" if emas.len() == 2 => {
                let mut it = emas.into_iter();
                IndicatorStateBox::Ppo(ppo::State {
                    short_ema: it.next().unwrap(),
                    long_ema: it.next().unwrap(),
                })
            }
            "macd" if emas.len() == 3 => {
                let mut it = emas.into_iter();
                IndicatorStateBox::Macd(macd::State {
                    short_ema: it.next().unwrap(),
                    long_ema: it.next().unwrap(),
                    signal_state: it.next().unwrap(),
                })
            }
            _ => IndicatorStateBox::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::{exec, finalize, Planable};
    use crate::indicator_types::Indicator;
    use crate::indicators::apo::Apo;
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;

    fn series() -> Vec<f64> {
        (0..200)
            .map(|i| ((i as f64) * 0.7).sin() * 10.0 + 100.0)
            .collect()
    }

    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(a[i].to_bits(), b[i].to_bits(), "{what}: mismatch at {i}");
        }
    }

    /// `CompiledPlan::run` == the proven `exec`+`finalize` path (streaming interp
    /// reproduces the reference interpreter bit-for-bit).
    #[test]
    fn query_run_matches_exec_finalize() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();

        let (res, _qs) = plan.run(&s);

        // Reference: exec + finalize on the same arena.
        let t = exec(plan.arena(), &s);
        let (_m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
            Ppo::recipe(&[12.0, 26.0]),
        ]);
        for (req, rr) in r.iter().enumerate() {
            let want = finalize(&t, &rr.outputs, &rr.output_leads);
            for out in 0..rr.outputs.len() {
                cmp_exact(res.request(req).output(out), &want[out], "vs_exec");
            }
        }
    }

    /// `run(first half) ++ resume(second half) == run(full)` for every
    /// output/optional. `run` gates on the *absolute* bar index, so the first
    /// run publishes from its lead onward and `resume` continues that line at
    /// the next bar — concatenating them must reproduce the single full pass.
    #[test]
    fn query_resume_matches_full_run() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();

        let (full, _) = plan.run(&s);

        let first: [&[f64]; 1] = [&close[..120]];
        let second: [&[f64]; 1] = [&close[120..]];
        let (head, qs) = plan.run(&first);
        let (tail, _) = plan.resume(qs, &second);

        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                let mut combined = head.outputs[req][out].clone();
                combined.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&combined, &full.outputs[req][out], "stream_eq_full");
            }
            for out in 0..full.optional[req].len() {
                let mut combined = head.optional[req][out].clone();
                combined.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(
                    &combined,
                    &full.optional[req][out],
                    "stream_eq_full_optional",
                );
            }
        }
    }

    /// `get_states` yields concrete states usable for independent streaming.
    #[test]
    fn get_states_projects_indicators() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();
        let (_, qs) = plan.run(&s);
        let states = plan.get_states(&qs);
        assert!(matches!(states[0], IndicatorStateBox::Apo(_)));
        assert!(matches!(states[1], IndicatorStateBox::Macd(_)));
        assert!(matches!(states[2], IndicatorStateBox::Ppo(_)));
    }

    /// THE wiring test for the packed executor: a dense dashboard — 12
    /// distinct-period EMAs on `close` (bucketed `Simd<4>`×3, or `Simd<8>+
    /// Simd<4>` under `avx512`) plus two scalar signal EMAs on different
    /// lines — satisfies `PackedPlan::worth_it`, so `run`/`resume` must take
    /// the packed path and still reproduce the `exec`+`finalize` oracle
    /// bit-for-bit, with the chunked stream re-concatenating to the full run.
    #[test]
    fn query_packed_dense_dashboard_matches_oracle() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Apo::recipe(&[5.0, 8.0]))
            .add(Apo::recipe(&[6.0, 9.0]))
            .add(Ppo::recipe(&[9.0, 25.0]))
            .add(Ppo::recipe(&[10.0, 26.0]))
            .add(Macd::recipe(&[11.0, 23.0, 17.0]))
            .add(Macd::recipe(&[12.0, 24.0, 9.0]))
            .build();

        let (res, _) = plan.run(&s);

        // Oracle: exec + finalize on the same arena.
        let t = crate::cse::exec(plan.arena(), &s);
        let (_m, r) = crate::cse::merge(&[
            Apo::recipe(&[5.0, 8.0]),
            Apo::recipe(&[6.0, 9.0]),
            Ppo::recipe(&[9.0, 25.0]),
            Ppo::recipe(&[10.0, 26.0]),
            Macd::recipe(&[11.0, 23.0, 17.0]),
            Macd::recipe(&[12.0, 24.0, 9.0]),
        ]);
        for (req, rr) in r.iter().enumerate() {
            let want = finalize(&t, &rr.outputs, &rr.output_leads);
            for out in 0..rr.outputs.len() {
                cmp_exact(res.request(req).output(out), &want[out], "dense_vs_oracle");
            }
            let want_opt = finalize(&t, &rr.optional_outputs, &rr.optional_output_leads);
            for out in 0..rr.optional_outputs.len() {
                cmp_exact(
                    res.request(req).optional(out),
                    &want_opt[out],
                    "dense_vs_oracle_optional",
                );
            }
        }

        // Streamed run must equal the full run (packed resume round-trip).
        let first: [&[f64]; 1] = [&close[..90]];
        let second: [&[f64]; 1] = [&close[90..150]];
        let third: [&[f64]; 1] = [&close[150..]];
        let (head, qs) = plan.run(&first);
        let (mid, qs) = plan.resume(qs, &second);
        let (tail, _) = plan.resume(qs, &third);
        for req in 0..res.outputs.len() {
            for out in 0..res.outputs[req].len() {
                let mut c = head.outputs[req][out].clone();
                c.extend_from_slice(&mid.outputs[req][out]);
                c.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&c, &res.outputs[req][out], "dense_stream_eq_full");
            }
        }
    }

    /// The optional-output switch (`Request::add_opt`): flags prune ONLY
    /// emission — enabled arrays and primaries are bit-identical to the
    /// all-on run; disabled slots stay in declared position as EMPTY arrays
    /// (hand `::indicator(None)` shape parity); primary-only mode matches the
    /// hand calls exactly.
    #[test]
    fn optional_switch_prunes_emission_not_values() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];

        // All-on (default `add`) — the reference emission set.
        let full = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();
        let (rf, _) = full.run(&s);

        // Primary-only (hand `None` parity).
        let none = Request::new()
            .add_opt(Apo::recipe(&[12.0, 26.0]), None)
            .add_opt(Macd::recipe(&[12.0, 26.0, 9.0]), None)
            .add_opt(Ppo::recipe(&[12.0, 26.0]), None)
            .build();
        let (rn, _) = none.run(&s);

        // Hand parity on primaries.
        let apo = Apo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        let macd = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0;
        let ppo = Ppo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        cmp_exact(rn.request(0).output(0), &apo[0], "hand_apo");
        cmp_exact(rn.request(1).output(0), &macd[0], "hand_macd_line");
        cmp_exact(rn.request(1).output(1), &macd[1], "hand_macd_signal");
        cmp_exact(rn.request(1).output(2), &macd[2], "hand_macd_hist");
        cmp_exact(rn.request(2).output(0), &ppo[0], "hand_ppo");

        // Pruning: declared slots remain (empty), primaries untouched,
        // all-on run still prints data everywhere.
        for req in 0..3 {
            assert_eq!(
                rn.optional[req].len(),
                rf.optional[req].len(),
                "req {req}: declared slots kept"
            );
            for out in 0..rn.optional[req].len() {
                assert!(
                    rn.optional[req][out].is_empty(),
                    "req {req} opt {out}: disabled must print EMPTY"
                );
                assert!(
                    !rf.optional[req][out].is_empty(),
                    "all-on req {req} opt {out} empty"
                );
            }
            for out in 0..rf.outputs[req].len() {
                cmp_exact(
                    &rn.outputs[req][out],
                    &rf.outputs[req][out],
                    "primary_invariance",
                );
            }
        }

        // Partial: all-on apo, all-off macd, ppo short_ema only → enabled
        // arrays bit-identical to the all-on run, disabled slots empty.
        let part = Request::new()
            .add_opt(Apo::recipe(&[12.0, 26.0]), Some(&[true, true]))
            .add_opt(Macd::recipe(&[12.0, 26.0, 9.0]), Some(&[false, false]))
            .add_opt(Ppo::recipe(&[12.0, 26.0]), Some(&[true, false]))
            .build();
        let (rp, _) = part.run(&s);
        cmp_exact(
            rp.request(2).optional(0),
            rf.request(2).optional(0),
            "partial_ppo_short_eq_full",
        );
        assert!(
            rp.request(2).optional(1).is_empty(),
            "ppo long_ema disabled"
        );
        assert!(
            rp.request(1).optional(0).is_empty() && rp.request(1).optional(1).is_empty(),
            "macd optionals disabled"
        );
        cmp_exact(
            rp.request(0).optional(0),
            rf.request(0).optional(0),
            "apo_opt0",
        );
        cmp_exact(
            rp.request(0).optional(1),
            rf.request(0).optional(1),
            "apo_opt1",
        );

        // Resumed streaming respects the same switches (the fence < 16 bars
        // exercises the fused engine; the tail chunk exercises columnar).
        let first: [&[f64]; 1] = [&close[..9]];
        let second: [&[f64]; 1] = [&close[9..]];
        let (head, qs) = none.run(&first);
        let (tail, _) = none.resume(qs, &second);
        for req in 0..3 {
            for out in 0..rn.outputs[req].len() {
                let mut c = head.outputs[req][out].clone();
                c.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&c, &rn.outputs[req][out], "streamed_primary_eq_fullrun");
            }
            for out in 0..rn.optional[req].len() {
                assert!(head.optional[req][out].is_empty() && tail.optional[req][out].is_empty());
            }
        }
    }
}
