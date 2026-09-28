//! `Request` → `JitPlan` → `run`/`resume`/`get_states` — the query surface
//! for the JIT executor (planner-owned interface compiled into one fused
//! machine loop, closed tail set inlined as CLIF ops).
//!
//! * [`Request`] builds the recipe list. `add` emits every declared optional;
//!   `add_opt` mirrors hand `::indicator(optional_outputs)` (`None` ⇒
//!   primary-only: disabled optionals print EMPTY arrays in position, their
//!   sinks are excluded from emission and from `first_bar`-independent lead
//!   clamping).
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
use crate::cse::Recipe;
use crate::indicators::ema::State as EmaState;
use crate::types::Warm;

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

    /// Append a request (from e.g. `Macd::recipe(&[12.,26.,9.])`). Every
    /// declared optional output emits (full `Results`).
    pub fn add(mut self, recipe: Recipe) -> Self {
        let flags = vec![true; recipe.optional_outputs.len()];
        self.recipes.push(recipe);
        self.opt_flags.push(flags);
        self
    }

    /// Append with hand-`::indicator`-parity optional-output flags: `None` ⇒
    /// every optional off (primary-only emission, the hand `None` call);
    /// `Some(flags)` ⇒ per declared optional in `Info::optional_outputs`
    /// order, missing tail = off. Disabled optionals print EMPTY arrays in
    /// declared position; enabled arrays and primaries are bit-identical to
    /// the all-on run.
    pub fn add_opt(mut self, recipe: Recipe, optional_outputs: Option<&[bool]>) -> Self {
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

    /// Merge (CSE) the interface graphs, bake leads, apply the optional
    /// switch, and compile the fused JIT loop. Panics on arena shapes the
    /// emitter does not cover yet (reserved node kinds, unknown tail kinds)
    /// — those recipes are the emitter's next coverage, not a fallback path.
    pub fn build(self) -> JitPlan {
        JitPlan::compile(&self)
    }
}

/// Outputs for every request in a run/resume, request-major.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// `outputs[request][output]` — primary, in `Info::outputs` order.
    pub outputs: Vec<Vec<Vec<f64>>>,
    /// `optional[request][output]` — `Info::optional_outputs` order; EMPTY
    /// for sinks disabled via [`Request::add_opt`].
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

/// Concrete hand `IndicatorState`s for DB/FFI/legacy streaming, exported
/// one-way (never fed back into `resume` — independent timelines).
pub enum IndicatorStateBox {
    /// An `Ema` mirror request: exactly the arena node's own interface state.
    Ema(EmaState<Warm>),
    Apo(crate::indicators::apo::State<Warm>),
    Macd(crate::indicators::macd::State<Warm>),
    Ppo(crate::indicators::ppo::State<Warm>),
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
            .add(Apo::recipe(&APO))
            .add(Ppo::recipe(&PPO))
            .add(Macd::recipe(&MACD))
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
        // ppo: [ppo] + optional [short, clamped long]
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

        for split in [90usize, 140, 20, 33, 25] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (head, qs) = plan.run(&s1);
            let (tail, _) = plan.resume(qs, &s2);
            for req in 0..full.outputs.len() {
                for out in 0..full.outputs[req].len() {
                    let mut c = head.outputs[req][out].clone();
                    c.extend_from_slice(&tail.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &full.outputs[req][out],
                        &format!("split{split} primary"),
                    );
                }
                for out in 0..full.optional[req].len() {
                    let mut c = head.optional[req][out].clone();
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
            .add_opt(Apo::recipe(&APO), None)
            .add_opt(Ppo::recipe(&PPO), None)
            .add_opt(Macd::recipe(&MACD), None)
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
            .add_opt(Apo::recipe(&APO), None)
            .add_opt(Ppo::recipe(&PPO), None)
            .add_opt(Macd::recipe(&MACD), Some(&[true, false]))
            .build();
        let (rp, _) = part.run(&s);
        cmp_exact(
            rp.request(2).optional(0),
            all.request(2).optional(0),
            "macd_short_enabled_eq_all",
        );
        assert!(rp.request(2).optional(1).is_empty(), "macd_long disabled");
    }

    /// `get_states` reconstitutes the hand `State` from BOTH authorities:
    /// interface EMA state from the arena, macd's signal from the tail — and
    /// each field's `ema` must equal the last *published* value on its line
    /// (shared arena node ⇒ identical across the three requests: one
    /// authority per node).
    #[test]
    fn get_states_restores_from_both_authorities() {
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
            .add_opt(Ema::recipe(&[12.0]), None)
            .add_opt(Ema::recipe(&[26.0]), None)
            .build();
        assert_eq!(
            plan.arena().nodes.len(),
            3,
            "ema requests must merge into the classic arena (Input + ema12 + ema26)"
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
        let mut c = h1.outputs[3][0].clone();
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
