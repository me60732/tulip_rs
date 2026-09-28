//! Columnar execution — the runtime "typed fused kernel"'s cousin.
//!
//! Transposes the fused interpreter's loop nest. [`fuse::FusedPlan`] walks
//! `for bar { for node { dispatch } }`; this walks `for node (topo order) {
//! one monomorphic loop over the whole chunk }`, so the per-node dispatch
//! happens ONCE per chunk instead of once per bar. Elementwise nodes
//! (`Sub/Mul/Div/Max`, `Const`, `Input`) become tight slice loops the
//! compiler autovectorizes; `Ema` becomes the hand-`init_state` recurrence
//! loop. This is the same per-bar dependency structure, transposed — and it
//! is bit-identical to [`fuse::FusedPlan`] (asserted in the `columnar_*`
//! tests):
//!
//! - An EMA's state trajectory is per-node sequential, so bar-major vs
//!   node-major processing agree on every value (the arena is topologically
//!   ordered, so a node's producers are computed before it).
//! - The producer-`NaN` freeze and the per-node defined-from lead gate are
//!   applied elementwise, exactly as `fuse` applies them per bar. The gate
//!   still lets the EMA state advance during gated bars (the seed/freeze
//!   must not depend on the node's own lead).
//! - A sink's output is just its node's column sliced from the sink's
//!   publish lead — the prologue/main distinction of `fuse` disappears,
//!   because every column value already carries the node's own gate and the
//!   sink lead dominates the node lead.
//!
//! All columns live in ONE scratch buffer (`node id → scratch[id*len ..]`),
//! every writer fills all `[0, len)` slots exactly once (so the buffer is
//! `uninit_vec!`d), and the bit-exact tests are the canary for a missed
//! slot — uninitialized garbage would fail them. This keeps a 5000-bar
//! advance at ~n·len·8 bytes of scratch + the owned `Results` — same memory
//! class as `fuse`, one allocation instead of seven.

use crate::cse::{fuse, Id, Merged, Node, QueryState, Resolved, Results};
use std::slice;

/// Tile width in bars: the per-node scratch is sized to
/// `arena_len × min(TILE, chunk_len)` f64s so a whole tile stays L1-resident
/// (e.g. a 12-node arena → 24KB). Node-major-within-tile is bit-identical to
/// node-major-over-the-whole-chunk because each node's recurrence is strictly
/// sequential and its state persists across tiles in [`QueryState`].
const TILE: usize = 256;

/// One reporting sink — same shape/publish rule as `fuse::Sink`
/// (`max(declared, node_lead)`).
#[derive(Debug, Clone, Copy)]
struct Sink {
    node: Id,
    lead: Id,
    /// Optional-output switch (see [`crate::cse::Resolved::optional_enabled`]).
    /// Disabled optional sinks keep their slot in `Results` but print an EMPTY
    /// array; the tile `lo < hi` write guard skips them via `rel = usize::MAX`.
    /// Primary sinks are always enabled.
    enabled: bool,
}

/// The columnar executor: arena + leads + the sink wiring, frozen once at
/// `Request::build` time. Data-length-independent.
pub(crate) struct ColumnarPlan {
    arena: Merged,
    /// Per-node defined-from lead (absolute bar), for the gate fold.
    leads: Vec<Id>,
    primary: Vec<Sink>,
    optional: Vec<Sink>,
    /// Per-request (primary, optional) output counts, to regroup the flat
    /// sink vectors into `Results`' request-major shape.
    counts: Vec<(usize, usize)>,
}

impl ColumnarPlan {
    /// Bake the plan: clone the arena, pin the leads, wire the sinks. The
    /// sink wiring is copied from [`fuse::FusedPlan::build`] (each executor
    /// is self-contained; `fuse` stays the untouched oracle).
    pub(crate) fn build(arena: &Merged, leads: &[Id], resolved: &[Resolved]) -> Self {
        let mut primary: Vec<Sink> = Vec::new();
        let mut optional: Vec<Sink> = Vec::new();
        let mut counts: Vec<(usize, usize)> = Vec::with_capacity(resolved.len());
        for res in resolved {
            for (oi, &node) in res.outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res.output_leads.get(oi).copied().unwrap_or(node_lead);
                primary.push(Sink {
                    node,
                    lead: node_lead.max(declared),
                    enabled: true,
                });
            }
            for (oi, &node) in res.optional_outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res
                    .optional_output_leads
                    .get(oi)
                    .copied()
                    .unwrap_or(node_lead);
                let enabled = res.optional_enabled.get(oi).copied().unwrap_or(true);
                optional.push(Sink {
                    node,
                    lead: node_lead.max(declared),
                    enabled,
                });
            }
            counts.push((res.outputs.len(), res.optional_outputs.len()));
        }
        ColumnarPlan {
            arena: arena.clone(),
            leads: leads.to_vec(),
            primary,
            optional,
            counts,
        }
    }

    /// Columnar streaming interpreter over one chunk, blocked into L1-sized
    /// tiles: per tile, per node (topo order), one monomorphic loop writes that
    /// node's tile column; sinks then copy the tile slices. Node-major-within-
    /// tile is bit-identical to the whole-chunk walk (the recurrence is
    /// sequential and the state persists across tiles in `QueryState`), and
    /// bit-identical to `FusedPlan::advance` (header + tests).
    pub(crate) fn advance(&self, state: &mut QueryState, series: &[&[f64]]) -> Results {
        let new_len = series.iter().map(|s| s.len()).max().unwrap_or(0);
        if new_len == 0 {
            return Results::default();
        }
        let start = state.next_bar;
        let n = self.arena.nodes.len();

        // Pre-sized sink vectors with the relative publish lead, exactly as
        // `FusedPlan::advance` (lengths `new_len − rel_lead`, saturating):
        // each tile writes its slice of the sink arrays by index, once.
        let mut p_out: Vec<Vec<f64>> = Vec::with_capacity(self.primary.len());
        let mut p_rel: Vec<usize> = Vec::with_capacity(self.primary.len());
        for s in &self.primary {
            let rel = s.lead.saturating_sub(start) as usize;
            p_out.push(crate::uninit_vec!(f64, new_len.saturating_sub(rel)));
            p_rel.push(rel);
        }
        let mut o_out: Vec<Vec<f64>> = Vec::with_capacity(self.optional.len());
        let mut o_rel: Vec<usize> = Vec::with_capacity(self.optional.len());
        for s in &self.optional {
            // Disabled optional sink: EMPTY array in declared position;
            // `pr = usize::MAX` makes every tile's `lo < hi` write guard skip it.
            if !s.enabled {
                o_out.push(Vec::new());
                o_rel.push(usize::MAX);
                continue;
            }
            let rel = s.lead.saturating_sub(start) as usize;
            o_out.push(crate::uninit_vec!(f64, new_len.saturating_sub(rel)));
            o_rel.push(rel);
        }

        // Per-node tile columns: node `id` at `base[id*tw .. id*tw + tlen]`.
        // Every writer fills its `tlen` slots before any consumer reads them
        // (topo order within the tile), so an uninitialized scratch is sound;
        // the bit-exact tests are the canary for a missed slot.
        let tw = TILE.min(new_len);
        let mut scratch = crate::uninit_vec!(f64, n * tw);
        let base = scratch.as_mut_ptr();

        let mut tile_start = 0usize;
        while tile_start < new_len {
            let tlen = tw.min(new_len - tile_start);

            for id in 0..n {
                let lead = self.leads[id];
                // Row `i` of this tile is global rel `tile_start + i`; bars
                // before the node's defined-from stay NaN. `start` may already
                // be past the lead (resume) ⇒ no gating.
                let lead_rel = lead.saturating_sub(start) as usize;
                let gate = lead_rel.saturating_sub(tile_start).min(tlen);
                // SAFETY: disjoint columns — stride `tw`, width `tlen` ≤ `tw`;
                // a node's inputs are strictly lower arena ids (topo arena).
                let col = unsafe { slice::from_raw_parts_mut(base.add(id * tw), tlen) };

                match self.arena.nodes[id] {
                    Node::Input { stream } => {
                        // Lead is 0 for Input, so no gate: copy the stream's
                        // bars for this tile (chunk-relative, like fuse), NaN
                        // beyond the stream's length.
                        let src = series.get(stream as usize).copied().unwrap_or(&[]);
                        let avail = src.len().saturating_sub(tile_start).min(tlen);
                        for i in 0..avail {
                            unsafe {
                                *col.get_unchecked_mut(i) = *src.get_unchecked(tile_start + i)
                            };
                        }
                        for i in avail..tlen {
                            col[i] = f64::NAN;
                        }
                    }
                    Node::Const { bits } => {
                        let c = f64::from_bits(bits);
                        for i in 0..tlen {
                            col[i] = c;
                        }
                    }
                    Node::Ema { src, period } => {
                        let prod = unsafe { col_slice(base, src as usize, tw, tlen) };
                        let e = &mut state.ema[id];
                        for i in 0..tlen {
                            let p = unsafe { *prod.get_unchecked(i) };
                            // Producer NaN ⇒ not yet reported: freeze the
                            // state (fuse's `eval` early-return), not just
                            // NaN the cell.
                            let v = if p.is_nan() {
                                f64::NAN
                            } else {
                                fuse::advance_ema(e, p, period)
                            };
                            col[i] = if i < gate { f64::NAN } else { v };
                        }
                    }
                    Node::Sub { a, b } => binop(
                        col,
                        unsafe { col_slice(base, a as usize, tw, tlen) },
                        unsafe { col_slice(base, b as usize, tw, tlen) },
                        gate,
                        |x, y| x - y,
                    ),
                    Node::Mul { a, b } => binop(
                        col,
                        unsafe { col_slice(base, a as usize, tw, tlen) },
                        unsafe { col_slice(base, b as usize, tw, tlen) },
                        gate,
                        |x, y| x * y,
                    ),
                    Node::Div { a, b } => binop(
                        col,
                        unsafe { col_slice(base, a as usize, tw, tlen) },
                        unsafe { col_slice(base, b as usize, tw, tlen) },
                        gate,
                        |x, y| x / y,
                    ),
                    // f64::max (NaN-propagates-other), matching ppo.rs `.max(EPSILON)`.
                    Node::Max { a, b } => binop(
                        col,
                        unsafe { col_slice(base, a as usize, tw, tlen) },
                        unsafe { col_slice(base, b as usize, tw, tlen) },
                        gate,
                        f64::max,
                    ),
                    // Filter/Hilbert kinds: not executed yet (no recipe uses them).
                    _ => col.fill(f64::NAN),
                }
            }

            // Sink writes for this tile: rows `[max(p_rel, tile_start),
            // tile_start + tlen)` land at `rel − p_rel` in the sink array —
            // identical values to fuse's prologue-gated + main writes.
            for (si, s) in self.primary.iter().enumerate() {
                let pr = p_rel[si];
                let lo = tile_start.max(pr);
                let hi = tile_start + tlen;
                if lo < hi {
                    let col = unsafe { col_slice(base, s.node as usize, tw, tlen) };
                    // SAFETY: `hi ≤ new_len ≤ pr + p_out[si].len()` and
                    // `lo ≥ pr` by the clamps above.
                    let dst = unsafe { p_out[si].get_unchecked_mut((lo - pr)..(hi - pr)) };
                    dst.copy_from_slice(&col[(lo - tile_start)..(hi - tile_start)]);
                }
            }
            for (si, s) in self.optional.iter().enumerate() {
                let pr = o_rel[si];
                let lo = tile_start.max(pr);
                let hi = tile_start + tlen;
                if lo < hi {
                    let col = unsafe { col_slice(base, s.node as usize, tw, tlen) };
                    let dst = unsafe { o_out[si].get_unchecked_mut((lo - pr)..(hi - pr)) };
                    dst.copy_from_slice(&col[(lo - tile_start)..(hi - tile_start)]);
                }
            }

            tile_start += tw;
        }

        state.next_bar = start + new_len as Id;

        // Regroup the flat sink vectors into `Results`' request-major shape.
        let mut out_requests = Vec::with_capacity(self.counts.len());
        let mut opt_requests = Vec::with_capacity(self.counts.len());
        for &(np, no) in &self.counts {
            out_requests.push(p_out.drain(..np).collect());
            opt_requests.push(o_out.drain(..no).collect());
        }
        Results {
            outputs: out_requests,
            optional: opt_requests,
        }
    }
}

/// Immutable view of the column at `id` (stride `tw`, width `tlen`) inside the
/// scratch buffer.
///
/// SAFETY contract (call sites): `id` is a lower arena id than every live
/// `&mut` column (topo arena), so the ranges are disjoint;
/// `(id + 1) * tw ≤ scratch len`.
#[inline]
unsafe fn col_slice<'a>(base: *mut f64, id: usize, tw: usize, tlen: usize) -> &'a [f64] {
    slice::from_raw_parts(base.add(id * tw), tlen)
}

/// One elementwise node column: NaN before the node's defined-from (`gate`),
/// then the op over two producer columns.
///
/// SAFETY: `out`, `a`, `b` are the same-width tile columns; `i < out.len()`.
#[inline]
fn binop(out: &mut [f64], a: &[f64], b: &[f64], gate: usize, f: fn(f64, f64) -> f64) {
    for i in 0..gate {
        out[i] = f64::NAN;
    }
    for i in gate..out.len() {
        unsafe {
            *out.get_unchecked_mut(i) = f(*a.get_unchecked(i), *b.get_unchecked(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::{exec, finalize, fuse::FusedPlan, merge, Planable};
    use crate::indicator_types::{Indicator, TState};
    use crate::indicators::apo::Apo;
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;
    use crate::types::Warm;

    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(a[i].to_bits(), b[i].to_bits(), "{what}: mismatch at {i}");
        }
    }

    fn cmp_results(a: &Results, b: &Results, what: &str) {
        assert_eq!(a.outputs.len(), b.outputs.len(), "{what}: request count");
        for req in 0..a.outputs.len() {
            assert_eq!(
                a.outputs[req].len(),
                b.outputs[req].len(),
                "{what}: req {req} outputs"
            );
            for out in 0..a.outputs[req].len() {
                cmp_exact(&a.outputs[req][out], &b.outputs[req][out], what);
            }
            assert_eq!(
                a.optional[req].len(),
                b.optional[req].len(),
                "{what}: req {req} optional"
            );
            for out in 0..a.optional[req].len() {
                cmp_exact(&a.optional[req][out], &b.optional[req][out], what);
            }
        }
    }

    /// `Apo(12,26) + Ppo(12,26) + Macd(12,26,9)` merged — resolved order:
    /// 0 = Apo, 1 = Ppo, 2 = Macd.
    fn merged_plan() -> (Merged, Vec<Resolved>, Vec<Id>) {
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Ppo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
        ]);
        let leads = m.leads();
        (m, r, leads)
    }

    fn long_series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
            .collect()
    }

    /// THE gate: the columnar executor is bit-identical to the fused
    /// per-bar interpreter (primary AND optional), to the `exec`+`finalize`
    /// oracle, and to the hand indicators (chained macd signal/histogram),
    /// on a 5000-bar run.
    #[test]
    fn columnar_matches_fused_and_oracle() {
        let close = long_series(5000);
        let s: [&[f64]; 1] = [close.as_slice()];
        let (m, r, leads) = merged_plan();
        let fused = FusedPlan::build(&m, &leads, &r);
        let col = ColumnarPlan::build(&m, &leads, &r);

        let mut qf = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let mut qc = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let a = fused.advance(&mut qf, &s);
        let b = col.advance(&mut qc, &s);
        cmp_results(&a, &b, "columnar_vs_fused");
        // The same data must also leave both engines' states identical.
        assert_eq!(a.outputs.len(), b.outputs.len());

        // Oracle: exec + finalize on the same arena.
        let t = exec(&m, &s);
        for (req, res) in r.iter().enumerate() {
            let want = finalize(&t, &res.outputs, &res.output_leads);
            for out in 0..res.outputs.len() {
                cmp_exact(&b.outputs[req][out], &want[out], "columnar_vs_oracle");
            }
            let want_opt = finalize(&t, &res.optional_outputs, &res.optional_output_leads);
            for out in 0..res.optional_outputs.len() {
                cmp_exact(
                    &b.optional[req][out],
                    &want_opt[out],
                    "columnar_vs_oracle_optional",
                );
            }
        }

        // Hand indicators.
        let apo = Apo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        let ppo = Ppo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        let macd = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0;
        cmp_exact(&b.outputs[0][0], &apo[0], "columnar_apo");
        cmp_exact(&b.outputs[1][0], &ppo[0], "columnar_ppo");
        cmp_exact(&b.outputs[2][0], &macd[0], "columnar_macd_line");
        cmp_exact(&b.outputs[2][1], &macd[1], "columnar_macd_signal");
        cmp_exact(&b.outputs[2][2], &macd[2], "columnar_macd_histogram");
    }

    /// Streaming contract: chunks split at ANY boundary re-concatenate to
    /// the single full run, bit-exact, with the state carried in the same
    /// caller-facing `QueryState`. Covers 90 (past the 33-bar lead) and the
    /// short 20-bar chunk where late sinks are empty in the head.
    #[test]
    fn columnar_resume_matches_full_run() {
        let close: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
            .collect();
        let (m, r, leads) = merged_plan();
        let col = ColumnarPlan::build(&m, &leads, &r);
        let s_all: [&[f64]; 1] = [close.as_slice()];
        let mut full_q = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let full = col.advance(&mut full_q, &s_all);

        // 90 / 140 split.
        let s1: [&[f64]; 1] = [&close[..90]];
        let s2: [&[f64]; 1] = [&close[90..140]];
        let s3: [&[f64]; 1] = [&close[140..]];
        let mut q = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let head = col.advance(&mut q, &s1);
        let mid = col.advance(&mut q, &s2);
        let tail = col.advance(&mut q, &s3);

        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                let mut c = head.outputs[req][out].clone();
                c.extend_from_slice(&mid.outputs[req][out]);
                c.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&c, &full.outputs[req][out], "split_vs_full_primary");
            }
            for out in 0..full.optional[req].len() {
                let mut c = head.optional[req][out].clone();
                c.extend_from_slice(&mid.optional[req][out]);
                c.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(&c, &full.optional[req][out], "split_vs_full_optional");
            }
        }

        // Short first chunk (20 < max_publish_lead 33): late sinks empty in
        // the head, resume completes them exactly.
        let s_h: [&[f64]; 1] = [&close[..20]];
        let s_rest: [&[f64]; 1] = [&close[20..]];
        let mut q = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let head = col.advance(&mut q, &s_h);
        let tail = col.advance(&mut q, &s_rest);
        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                let mut c = head.outputs[req][out].clone();
                c.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&c, &full.outputs[req][out], "short_chunk_primary");
            }
            for out in 0..full.optional[req].len() {
                let mut c = head.optional[req][out].clone();
                c.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(&c, &full.optional[req][out], "short_chunk_optional");
            }
        }
    }

    /// Tabular sanity of the direct bit-exactness, held for a single scalar
    /// EMA too (asserts `fuse::advance_ema` is what both paths share).
    #[test]
    fn columnar_ema_trajectory_matches_fused() {
        let data = long_series(500);
        let s: [&[f64]; 1] = [data.as_slice()];
        let mut recipe = crate::cse::Recipe::new();
        let input = recipe.add(Node::Input { stream: 0 });
        let e1 = recipe.add(Node::Ema {
            src: input,
            period: 12,
        });
        let e2 = recipe.add(Node::Ema { src: e1, period: 9 });
        recipe.outputs = vec![e1, e2];
        let (m, r) = merge(&[recipe]);
        let leads = m.leads();
        let fused = FusedPlan::build(&m, &leads, &r);
        let col = ColumnarPlan::build(&m, &leads, &r);
        let mut qf = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let mut qc = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let a = fused.advance(&mut qf, &s);
        let b = col.advance(&mut qc, &s);
        cmp_results(&a, &b, "chained_ema");
        // Hand reference for the chained terminal: e1 = ema12 (valid from bar
        // 11, seed = data[0]); e2 = ema9(e1) (seed from e1's first valid
        // value at bar 11). The last reported values must match.
        let mut e1 = crate::indicators::ema::State::new(data[0], 12).into_warm();
        let mut e2: Option<crate::indicators::ema::State<Warm>> = None;
        let mut e2_val = f64::NAN;
        for i in 1..data.len() {
            let v = e1.calc(data[i]);
            if i >= 11 {
                match &mut e2 {
                    Some(s) => e2_val = s.calc(v),
                    None => {
                        let s = crate::indicators::ema::State::new(v, 9).into_warm();
                        e2_val = s.ema;
                        e2 = Some(s);
                    }
                }
            }
        }
        assert_eq!(
            b.outputs[0][1].last().unwrap().to_bits(),
            e2_val.to_bits(),
            "chained ema(9) terminal via columnar"
        );
    }
}
