//! Fused execution path: the planner's equivalent of the hand-written
//! `init_state` prologue + aligned `cycle_*` loop.
//!
//! # Why this shape (mirrors `macd.rs` / the road train)
//! `CompiledPlan::run` must reproduce the hand indicators' *performance model*,
//! not just their math. In `macd.rs` every lead cost is paid once in
//! `init_state`: the EMAs advance, the `macd_line` head is written, and the
//! state is carried up to the bar where *all* outputs are writable. Then
//! `slice_outputs_start!` head-slices the longer producer so macd/signal/hist
//! share one index base, and `cycle_macd` runs a flat `for i` with zero `if`
//! and `get_unchecked_mut` index writes (no `push` headroom checks).
//!
//! [`FusedPlan::advance`] generalizes that to an arbitrary merged arena:
//! - **Build time** (`Request::build`): bake the flattened sink wiring
//!   (node id + publish lead per sink, request-major) and the
//!   `max_publish_lead` — the last sink to come online.
//! - **Prologue** (`rel ∈ [0, inline)`): the alignment region. Some nodes are
//!   still NaN before their defined-from lead (gated [`FusedPlan::eval`]);
//!   short-lead sinks already print while long-lead ones don't. This is the
//!   `init_state` equivalent.
//! - **Main loop** (`rel ∈ [inline, new_len)`): *every* node is defined and
//!   *every* sink is online, so each sink's write index is `rel − rel_lead`
//!   with no bounds checks, no `push`, no per-sink `if` — the `cycle_macd`
//!   shape. When the chunk ends *before* the last sink comes online the main
//!   range is empty by construction, so late sinks are never indexed.
//!
//! Streaming: a `resume` whose `start` is already past `max_publish_lead` gets
//! `inline == 0` — an empty prologue, straight into the fast loop. The shared
//! [`crate::cse::QueryState`] EMA authority is unchanged, so a node consumed
//! by many sinks still advances exactly once per bar (the CSE win).
//!
//! # Equivalence notes (vs the old gated interpreter, proven by the tests)
//! - Sink publish lead is `max(declared, node_defined_lead)`. `finalize` in
//!   `plan.rs` already clamps a declared lead below defined-from by slicing
//!   the node stream from its own start, so this matches the oracle for every
//!   recipe, and matches the old interpreter for all *valid* recipes
//!   (publish lead ≥ defined-from, e.g. APO declares 26 over a node defined
//!   at 25).
//! - The ungated `eval_steady` is only reached at `rel ≥ inline`, where
//!   `inline = max_publish_lead − start` is ≥ every node's defined-from lead
//!   (sink leads dominate the node leads they report, and arena leads are
//!   monotonic along edges), so the per-node `bar < lead` gate is provably a
//!   no-op there. The EMA `is_nan` producer check is KEPT in steady so even
//!   ragged multi-stream input matches the reference bit-for-bit.

use crate::cse::{Id, Merged, Node, QueryState, Resolved, Results};
use crate::indicator_types::TState;
use crate::indicators::ema::State as EmaState;
use crate::types::Warm;

/// One reporting sink: which arena node it prints, and the absolute bar it
/// starts printing at (publish lead, already clamped up to the node's
/// defined-from in [`FusedPlan::build`]).
#[derive(Debug, Clone, Copy)]
struct Sink {
    node: Id,
    lead: Id,
    /// Optional-output switch (see [`crate::cse::Resolved::optional_enabled`]).
    /// Disabled optional sinks still occupy their slot in `Results` but print
    /// an EMPTY array; primary sinks are always enabled.
    enabled: bool,
}

/// Precomputed fused execution plan. Built once at `Request::build()` time,
/// immutable and data-length-independent.
pub(crate) struct FusedPlan {
    /// Arena nodes (cloned for self-containment; static once built).
    arena: Merged,
    /// Per-node defined-from lead (absolute bar), used by the prologue's
    /// gated eval. The main loop needs no gates.
    leads: Vec<Id>,
    /// Primary sinks, request-major then output-major (matches `Results`).
    primary: Vec<Sink>,
    /// Optional sinks, same ordering.
    optional: Vec<Sink>,
    /// The last sink to come online across all requests.
    max_publish_lead: Id,
    /// Per-request (primary, optional) counts, to regroup the flattened sink
    /// vectors into `Results`' request-major shape.
    counts: Vec<(usize, usize)>,
}

impl FusedPlan {
    /// Bake the fused plan: flatten sink wiring (request-major) and record
    /// `max_publish_lead`. Everything here is data-length-independent.
    pub(crate) fn build(arena: &Merged, leads: &[Id], resolved: &[Resolved]) -> Self {
        let mut primary: Vec<Sink> = Vec::new();
        let mut optional: Vec<Sink> = Vec::new();
        let mut counts: Vec<(usize, usize)> = Vec::with_capacity(resolved.len());
        let mut max_publish_lead = 0u32;

        for res in resolved {
            for (oi, &node) in res.outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res.output_leads.get(oi).copied().unwrap_or(node_lead);
                let lead = node_lead.max(declared);
                primary.push(Sink {
                    node,
                    lead,
                    enabled: true,
                });
                max_publish_lead = max_publish_lead.max(lead);
            }
            for (oi, &node) in res.optional_outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res
                    .optional_output_leads
                    .get(oi)
                    .copied()
                    .unwrap_or(node_lead);
                let lead = node_lead.max(declared);
                let enabled = res.optional_enabled.get(oi).copied().unwrap_or(true);
                optional.push(Sink {
                    node,
                    lead,
                    enabled,
                });
                // Disabled sinks are excluded from `max_publish_lead`: the
                // gated prologue no longer stretches for them.
                if enabled {
                    max_publish_lead = max_publish_lead.max(lead);
                }
            }
            counts.push((res.outputs.len(), res.optional_outputs.len()));
        }

        FusedPlan {
            arena: arena.clone(),
            leads: leads.to_vec(),
            primary,
            optional,
            max_publish_lead,
            counts,
        }
    }

    /// Fused streaming interpreter: pre-sized index writes, prologue (the
    /// alignment region) then the gate-free main loop. Bit-identical to the
    /// scalar interpreter and the `exec`+`finalize` oracle.
    pub(crate) fn advance(&self, state: &mut QueryState, series: &[&[f64]]) -> Results {
        let new_len = series.iter().map(|s| s.len()).max().unwrap_or(0);
        if new_len == 0 {
            return Results::default();
        }
        let new_len = new_len as Id;
        let start = state.next_bar;
        // Relative bar at which the *last* sink comes online. `[0, inline)` is
        // the prologue; `[inline, new_len)` is the flat main loop. If the
        // chunk ends before that bar, `inline == new_len` and the main loop is
        // empty — so late sinks (write index `rel − rel_lead` undefined there)
        // are never touched.
        let inline = self.max_publish_lead.saturating_sub(start).min(new_len);

        // Pre-size every sink's output vector to its exact publish count
        // (`new_len − rel_lead`) with `uninit_vec!`, recording the relative
        // lead so writes are pure `get_unchecked_mut` index stores.
        let mut p_out: Vec<Vec<f64>> = Vec::with_capacity(self.primary.len());
        let mut p_rel: Vec<Id> = Vec::with_capacity(self.primary.len());
        for s in &self.primary {
            let rel = s.lead.saturating_sub(start);
            // saturating: a sink whose lead falls beyond this chunk's end
            // publishes nothing here (len 0), e.g. a 20-bar first run with
            // `max_publish_lead == 33`.
            p_out.push(crate::uninit_vec!(
                f64,
                new_len.saturating_sub(rel) as usize
            ));
            p_rel.push(rel);
        }
        let mut o_out: Vec<Vec<f64>> = Vec::with_capacity(self.optional.len());
        let mut o_rel: Vec<Id> = Vec::with_capacity(self.optional.len());
        for s in &self.optional {
            // Disabled optional sink: EMPTY array in declared position,
            // `rel = Id::MAX` so the gated prologue can never select it.
            if !s.enabled {
                o_out.push(Vec::new());
                o_rel.push(Id::MAX);
                continue;
            }
            let rel = s.lead.saturating_sub(start);
            o_out.push(crate::uninit_vec!(
                f64,
                new_len.saturating_sub(rel) as usize
            ));
            o_rel.push(rel);
        }

        let n = self.arena.nodes.len();
        let mut row = vec![f64::NAN; n];

        // ---- Prologue [0, inline): gated eval; sinks write once online. ----
        for rel in 0..inline {
            let bar = start + rel;
            for id in 0..n {
                row[id] = self.eval(id, bar, rel, series, state, &row);
            }
            for (si, s) in self.primary.iter().enumerate() {
                if rel >= p_rel[si] {
                    unsafe {
                        *p_out[si].get_unchecked_mut((rel - p_rel[si]) as usize) =
                            row[s.node as usize];
                    }
                }
            }
            for (si, s) in self.optional.iter().enumerate() {
                if rel >= o_rel[si] {
                    unsafe {
                        *o_out[si].get_unchecked_mut((rel - o_rel[si]) as usize) =
                            row[s.node as usize];
                    }
                }
            }
        }

        // ---- Main loop [inline, new_len): all nodes defined, all sinks
        // online (main non-empty ⇒ inline == max rel_lead ≥ every rel_lead),
        // so every write index `rel − rel_lead` is in-bounds by construction
        // and each slot is written exactly once. Branch-free, index-stores
        // only — the `cycle_macd` hot-loop shape. ----
        for rel in inline..new_len {
            for id in 0..n {
                row[id] = self.eval_steady(id, rel, series, state, &row);
            }
            for (si, s) in self.primary.iter().enumerate() {
                unsafe {
                    *p_out[si].get_unchecked_mut((rel - p_rel[si]) as usize) = row[s.node as usize];
                }
            }
            for (si, s) in self.optional.iter().enumerate() {
                // Disabled sinks printed an empty vec above; `rel − o_rel`
                // would underflow against `Id::MAX`, so skip unconditionally.
                if !s.enabled {
                    continue;
                }
                unsafe {
                    *o_out[si].get_unchecked_mut((rel - o_rel[si]) as usize) = row[s.node as usize];
                }
            }
        }

        state.next_bar = start + new_len;

        // Regroup the flat sink vectors into `Results`' request-major shape.
        let mut outputs = Vec::with_capacity(self.counts.len());
        let mut optional = Vec::with_capacity(self.counts.len());
        for &(np, no) in &self.counts {
            outputs.push(p_out.drain(..np).collect());
            optional.push(o_out.drain(..no).collect());
        }
        Results { outputs, optional }
    }

    /// Gated node eval (prologue only): per-node lead gate + EMA producer NaN
    /// check. Identical to the old scalar interpreter.
    fn eval(
        &self,
        id: usize,
        bar: u32,
        rel: u32,
        series: &[&[f64]],
        state: &mut QueryState,
        row: &[f64],
    ) -> f64 {
        let v = match self.arena.nodes[id] {
            Node::Input { stream } => series
                .get(stream as usize)
                .and_then(|s| s.get(rel as usize))
                .copied()
                .unwrap_or(f64::NAN),
            Node::Const { bits } => f64::from_bits(bits),
            Node::Ema { src, period } => {
                let p = row[src as usize];
                if p.is_nan() {
                    return f64::NAN; // producer not yet reported
                }
                advance_ema(&mut state.ema[id], p, period)
            }
            Node::Sub { a, b } => row[a as usize] - row[b as usize],
            Node::Mul { a, b } => row[a as usize] * row[b as usize],
            Node::Div { a, b } => row[a as usize] / row[b as usize],
            // f64::max (NaN-propagates-other), matching ppo.rs `.max(EPSILON)`.
            Node::Max { a, b } => row[a as usize].max(row[b as usize]),
            // Filter/Hilbert kinds: not executed yet (no recipe uses them).
            _ => f64::NAN,
        };
        // Gate to defined-from so consumers read producers only once reported.
        if bar < self.leads[id] {
            f64::NAN
        } else {
            v
        }
    }

    /// Ungated node eval (main loop): `rel >= inline` guarantees `bar` is past
    /// every node's defined-from, so the lead gate is a provable no-op. The
    /// EMA producer `is_nan` check stays (predictably false for aligned data)
    /// so even ragged multi-stream input is bit-identical to [`Self::eval`].
    fn eval_steady(
        &self,
        id: usize,
        rel: u32,
        series: &[&[f64]],
        state: &mut QueryState,
        row: &[f64],
    ) -> f64 {
        match self.arena.nodes[id] {
            Node::Input { stream } => series
                .get(stream as usize)
                .and_then(|s| s.get(rel as usize))
                .copied()
                .unwrap_or(f64::NAN),
            Node::Const { bits } => f64::from_bits(bits),
            Node::Ema { src, period } => {
                let p = row[src as usize];
                if p.is_nan() {
                    return f64::NAN;
                }
                advance_ema(&mut state.ema[id], p, period)
            }
            Node::Sub { a, b } => row[a as usize] - row[b as usize],
            Node::Mul { a, b } => row[a as usize] * row[b as usize],
            Node::Div { a, b } => row[a as usize] / row[b as usize],
            Node::Max { a, b } => row[a as usize].max(row[b as usize]),
            _ => f64::NAN,
        }
    }
}

/// Advance an arena EMA node by one reported bar: seed on its first valid
/// value, then `calc`. Shared by both eval paths so a deduped node advances
/// exactly once per bar regardless of how many sinks read it. Also the scalar
/// EMA lifecycle the packed executor (`simd.rs`) reuses for its leftover
/// scalar nodes.
#[inline(always)]
pub(crate) fn advance_ema(ema: &mut Option<EmaState<Warm>>, p: f64, period: Id) -> f64 {
    match ema {
        Some(e) => e.calc(p),
        None => {
            let e = EmaState::new(p, period as usize).into_warm();
            let out = e.ema;
            *ema = Some(e);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cse::{merge, Planable};
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

    /// Fused run matches the `exec`+`finalize` oracle, and a three-way stream
    /// split re-concatenates to the single full run (bit-exact, primary and
    /// optional). The first split point (90) is past `max_publish_lead` (33),
    /// so resumes 2–3 exercise the empty-prologue fast path.
    #[test]
    fn fused_run_matches_exec_finalize_split() {
        use crate::cse::Request;

        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];

        let plan = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();

        // Reference: exec + finalize on the same arena.
        let t = crate::cse::exec(plan.arena(), &s);
        let (_m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
            Ppo::recipe(&[12.0, 26.0]),
        ]);

        let (full_fused, _) = plan.run(&s);

        for (req_idx, res) in r.iter().enumerate() {
            let want = crate::cse::finalize(&t, &res.outputs, &res.output_leads);
            for out_idx in 0..res.outputs.len() {
                cmp_exact(
                    full_fused.request(req_idx).output(out_idx),
                    &want[out_idx],
                    "fused_vs_exec_primary",
                );
            }
            let want_opt =
                crate::cse::finalize(&t, &res.optional_outputs, &res.optional_output_leads);
            for out_idx in 0..res.optional_outputs.len() {
                cmp_exact(
                    full_fused.request(req_idx).optional(out_idx),
                    &want_opt[out_idx],
                    "fused_vs_exec_optional",
                );
            }
        }

        // Split run: 0..90, 90..140, 140..200.
        let first: [&[f64]; 1] = [&close[..90]];
        let second: [&[f64]; 1] = [&close[90..140]];
        let third: [&[f64]; 1] = [&close[140..]];

        let (head, qs) = plan.run(&first);
        let (mid, qs2) = plan.resume(qs, &second);
        let (tail, _) = plan.resume(qs2, &third);

        for req_idx in 0..full_fused.outputs.len() {
            for out_idx in 0..full_fused.outputs[req_idx].len() {
                let mut combined = head.outputs[req_idx][out_idx].clone();
                combined.extend_from_slice(&mid.outputs[req_idx][out_idx]);
                combined.extend_from_slice(&tail.outputs[req_idx][out_idx]);
                cmp_exact(
                    &combined,
                    &full_fused.outputs[req_idx][out_idx],
                    "split_vs_full_primary",
                );
            }
            for out_idx in 0..full_fused.optional[req_idx].len() {
                let mut combined = head.optional[req_idx][out_idx].clone();
                combined.extend_from_slice(&mid.optional[req_idx][out_idx]);
                combined.extend_from_slice(&tail.optional[req_idx][out_idx]);
                cmp_exact(
                    &combined,
                    &full_fused.optional[req_idx][out_idx],
                    "split_vs_full_optional",
                );
            }
        }
    }

    /// A chunk that ENDS before the last sink comes online (`new_len <
    /// max_publish_lead`): the main loop must be empty, short-lead sinks fill
    /// their heads in the prologue, late sinks stay zero-length, and a resume
    /// continues exactly. This pins the `inline`-clamp invariant.
    #[test]
    fn fused_short_chunk_then_resume() {
        use crate::cse::Request;

        let close = series();
        let plan = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]))
            .add(Macd::recipe(&[12.0, 26.0, 9.0]))
            .add(Ppo::recipe(&[12.0, 26.0]))
            .build();

        // 20 bars < max_publish_lead (33): macd signal/histogram not yet online.
        let first: [&[f64]; 1] = [&close[..20]];
        let rest: [&[f64]; 1] = [&close[20..]];
        let (head, qs) = plan.run(&first);
        let (tail, _) = plan.resume(qs, &rest);

        let s_all: [&[f64]; 1] = [close.as_slice()];
        let (full, _) = plan.run(&s_all);

        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                assert!(
                    head.outputs[req][out].len() + tail.outputs[req][out].len()
                        == full.outputs[req][out].len(),
                    "req {req} out {out}: head {} + tail {} != full {}",
                    head.outputs[req][out].len(),
                    tail.outputs[req][out].len(),
                    full.outputs[req][out].len(),
                );
                let mut combined = head.outputs[req][out].clone();
                combined.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(
                    &combined,
                    &full.outputs[req][out],
                    "short_chunk_split_primary",
                );
            }
            for out in 0..full.optional[req].len() {
                let mut combined = head.optional[req][out].clone();
                combined.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(
                    &combined,
                    &full.optional[req][out],
                    "short_chunk_split_optional",
                );
            }
        }
    }
}
