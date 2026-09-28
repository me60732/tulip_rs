//! Scalar (reference) interpreter + `lead` derivation over a merged arena.
//!
//! This is the "what does the merged graph compute, and when is each value
//! valid" oracle. SIMD lane packing is a *separate* lowering; this runs nodes
//! one reported-bar at a time and MUST be bit-identical to it
//! (`MATHEMATICAL_ENGINE.md` §5 `Exact`). The merged graph reproducing the
//! hand `Apo`/`Ppo`/`Macd` `::indicator()` outputs (test below) is the proof the
//! CSE merge is sound before we benchmark SIMD against it.
//!
//! # The §5.3 model implemented here
//! A node's `lead` (`Merged::leads`) = bars consumed before its first *reported*
//! value. Execution follows the catalog convention: an `Ema` is seeded from its
//! producer's first **reported** value, warm-up-advanced over `period` reported
//! values, then *reports* `value` per subsequent bar; arithmetic combines
//! producers at the max of their leads. The key distinction (and the trap): a
//! node's recurrence advances per reported bar, but it is only *emitted* from
//! its own `lead` — seeding on the producer's *reported* stream (not absolute
//! bar 0) is what makes *chained* EMAs (e.g. a signal line) reproduce the hand
//! code.
//!
//! # Known-pending: macd's hand `init_state` is bespoke
//! `lead(Ema)=lead(src)+(period-1)` (defined-from; `ema.rs::init_state` seeds
//! what `Merged::leads` implements, and it makes `Apo` and `Ppo` (and standalone
//! `Ema`) emit their reported arrays with matching lengths. `Macd::indicator`,
//! however, reports its `macd_line` from bar `long-1` and *seeds* the signal EMA
//! from that early `macd_line` value (bar `long-1`) — which is the defined-from
//! itself under `+(period-1)`, so `macd_line`'s per-bar *values* match (the
//! ema chain is identical), which the right-aligned value test confirms, but the
//! leading element count differs and the *chained* signal/histogram diverge
//! because their seed bar is off by one. Reconciling that is a catalog
//! `lead`/warm-up decision (PRIMITIVE_CATALOG / decision P), not an
//! interpreter fix; the chained outputs align on values and are now active.
//! Exact array *lengths* still need the per-sink publish-lead (`data_len -
//! capacity` via `slot_lengths`/`min_data`), tracked open in §11 / decision U.

use crate::cse::{Id, Merged, Node};
use crate::indicator_types::TState;
use crate::indicators::ema::State as EmaState;

/// One node's reported stream, with the absolute bar its index 0 lands on.
#[derive(Debug, Clone)]
pub struct NodeOut {
    /// First absolute bar that holds a valid (reported) value.
    pub start: Id,
    /// Reported values, `vals[k]` is the value at absolute bar `start + k`.
    pub vals: Vec<f64>,
}

/// Run the merged `arena` against `series` (raw input streams; index =
/// `Node::Input{stream}`). Returns one [`NodeOut`] per arena node, in arena
/// order; consumers find their request's outputs via the [`Merged`] ids + the
/// arena's [`leads`](Merged::leads).
pub fn exec(arena: &Merged, series: &[&[f64]]) -> Vec<NodeOut> {
    let total = series.iter().map(|s| s.len()).max().unwrap_or(0);
    let leads = arena.leads();
    let n = arena.nodes.len();
    let mut out: Vec<NodeOut> = vec![
        NodeOut {
            start: 0,
            vals: Vec::new()
        };
        n
    ];

    for id in 0..n {
        match arena.nodes[id] {
            Node::Input { stream } => {
                let v = series
                    .get(stream as usize)
                    .map(|c| c.to_vec())
                    .unwrap_or_default();
                out[id] = NodeOut { start: 0, vals: v };
            }
            Node::Const { bits } => {
                let c = f64::from_bits(bits);
                out[id] = NodeOut {
                    start: 0,
                    vals: vec![c; total],
                };
            }
            Node::Ema { src, period } => {
                let (src, period) = (src as usize, period as usize);
                let (pstart, pvals) = (out[src].start, &out[src].vals);
                // Need `period` producer values to reach the defined-from terminal.
                if pvals.len() < period {
                    out[id] = NodeOut {
                        start: pstart + (period as Id).saturating_sub(1),
                        vals: Vec::new(),
                    };
                    continue;
                }
                // Seed from the producer's first reported value; warm-up over
                // warm up over the next `period-1` values; the resulting terminal
                // (bar `pstart+period-1`) IS the first defined-from report — which
                // is exactly how macd seeds a *chained* EMA (`signal` from
                // `macd_line`'s first value). The standalone `Ema::indicator` drop
                // of this terminal belongs to the *publish*-lead (capacity), §5.3.
                let mut e = EmaState::new(pvals[0], period).into_warm();
                for k in 1..period {
                    e.calc(pvals[k]);
                }
                let mut vals = Vec::with_capacity(pvals.len() - period + 1);
                vals.push(e.ema);
                for &x in &pvals[period..] {
                    vals.push(e.calc(x));
                }
                out[id] = NodeOut {
                    start: pstart + (period as Id).saturating_sub(1),
                    vals,
                };
            }
            Node::Sub { a, b } => out[id] = binop(a as usize, b as usize, &out, |x, y| x - y),
            Node::Mul { a, b } => out[id] = binop(a as usize, b as usize, &out, |x, y| x * y),
            Node::Div { a, b } => out[id] = binop(a as usize, b as usize, &out, |x, y| x / y),
            Node::Max { a, b } => {
                // f64::max (NaN-propagates-other), matching ppo.rs:90 `.max(EPSILON)`.
                out[id] = binop(a as usize, b as usize, &out, |x, y| x.max(y))
            }
            // Filter / Hilbert kinds: not executed (no test recipe uses them yet).
            _ => {}
        }
    }
    // `leads` is the authoritative check; `out[*].start` must agree (see tests).
    debug_assert_eq!(leads.len(), n);
    for (o, &l) in out.iter().zip(&leads) {
        debug_assert!(o.start == l || o.vals.is_empty(), "exec/lead drift @ {l}");
    }
    out
}

/// Slice each sink's node stream down to that sink's publish lead, producing
/// arrays whose length matches the indicator's own `data_len - capacity`.
/// The arena stream starts at the node's *defined-from*; a sink that publishes
/// later (e.g. APO at 26 over a sub defined-from 25) drops the extra head here.
/// lead < defined-from (shouldn't happen) ⇒ whole stream.
pub fn finalize(out: &[NodeOut], ids: &[Id], leads: &[Id]) -> Vec<Vec<f64>> {
    ids.iter()
        .enumerate()
        .map(|(i, &id)| {
            let o = &out[id as usize];
            let lead = leads.get(i).copied().unwrap_or(o.start);
            let off = lead.saturating_sub(o.start) as usize;
            o.vals.get(off..).unwrap_or(&[]).to_vec()
        })
        .collect()
}

/// Combine two producers' reported streams where both are valid (max of starts).
fn binop(a: usize, b: usize, out: &[NodeOut], f: impl Fn(f64, f64) -> f64) -> NodeOut {
    let (sa, sb) = (out[a].start, out[b].start);
    let start = sa.max(sb);
    let end_a = sa + out[a].vals.len() as Id;
    let end_b = sb + out[b].vals.len() as Id;
    let end = end_a.min(end_b);
    let mut vals = Vec::new();
    if end > start {
        vals.reserve((end - start) as usize);
        for bar in start..end {
            vals.push(f(
                out[a].vals[(bar - sa) as usize],
                out[b].vals[(bar - sb) as usize],
            ));
        }
    }
    NodeOut { start, vals }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::{merge, Planable};
    use crate::indicator_types::Indicator;
    use crate::indicators::apo::Apo;
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;

    fn series() -> Vec<f64> {
        (0..200)
            .map(|i| ((i as f64) * 0.7).sin() * 10.0 + 100.0)
            .collect()
    }

    /// Exact, full-array bit equality (no truncation).
    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(a[i].to_bits(), b[i].to_bits(), "{what}: mismatch at {i}");
        }
    }

    #[test]
    fn lead_matches_hand_defined_from() {
        // Defined-from convention: lead(Ema)=lead(src)+(period-1) — matches
        // macd's hand init_state seeding (bar long-1). Asserted against the
        // hand indicators' own lead math (§5.3).
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
            Ppo::recipe(&[12.0, 26.0]),
        ]);
        let l = m.leads();
        // APO: ema12=11, ema26=25, sub=25. (apo publishes from bar 26 via its
        // own min_data/sink-lead; the *node* is defined-from 25.)
        assert_eq!(l[r[0].outputs[0] as usize], 25, "apo_line defined-from");
        // macd_line = max(ema12=11, ema26=25) = 25 → matches hand's report at
        // long-1=25 exactly (no discrepancy under defined-from).
        assert_eq!(l[r[1].outputs[0] as usize], 25, "macd_line defined-from");
        // signal = ema(macd_line,9): 25+(9-1)=33 → matches hand's report at 33.
        assert_eq!(l[r[1].outputs[1] as usize], 33, "signal defined-from");
        assert_eq!(l[r[1].outputs[2] as usize], 33, "histogram defined-from");
        // PPO primary = div(ema-fed) → 25.
        assert_eq!(l[r[2].outputs[0] as usize], 25, "ppo defined-from");
    }

    #[test]
    fn merged_exec_matches_hand_indicators_ema_of_price() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
            Ppo::recipe(&[12.0, 26.0]),
        ]);
        let t = exec(&m, &s);

        // Publish-lead slice (apo primary leads [26]) drops the extra head bar →
        // exact, full-array equality vs the hand indicator.
        let apo_p = finalize(&t, &r[0].outputs, &r[0].output_leads);
        let apo = Apo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        cmp_exact(&apo_p[0], &apo[0], "apo");

        let ppo_p = finalize(&t, &r[2].outputs, &r[2].output_leads);
        let ppo = Ppo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        cmp_exact(&ppo_p[0], &ppo[0], "ppo");

        // macd_line publish lead == its defined-from (25), so finalize slices
        // 0 and this is the whole stream.
        let macd_p = finalize(&t, &r[1].outputs, &r[1].output_leads);
        let macd = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0;
        cmp_exact(&macd_p[0], &macd[0], "macd_line");
    }

    /// Chained-EMA outputs (macd `signal`/`histogram`). With defined-from
    /// `lead(Ema)=lead(src)+(period-1)`, the interpreter seeds `signal` from
    /// `macd_line`'s first defined value at bar `long-1` (25) and reports from
    /// `25+(9-1)=33` — matching hand `Macd::init_state` exactly (hand seeds the
    /// signal from that early value too). This is now active (was `#[ignore]`d
    /// under the old `+period` rule that put `signal` at 35).
    #[test]
    fn merged_exec_matches_chained_ema_macd() {
        let close = series();
        let s: [&[f64]; 1] = [close.as_slice()];
        let (m, r) = merge(&[Macd::recipe(&[12.0, 26.0, 9.0])]);
        let t = exec(&m, &s);
        let macd = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0;
        // Publish leads match defined-from ([25,33,33]) so finalize is a no-op
        // slice here; exact full arrays vs the hand indicator.
        let mp = finalize(&t, &r[0].outputs, &r[0].output_leads);
        cmp_exact(&mp[0], &macd[0], "macd_line");
        cmp_exact(&mp[1], &macd[1], "signal");
        cmp_exact(&mp[2], &macd[2], "histogram");
    }
}
