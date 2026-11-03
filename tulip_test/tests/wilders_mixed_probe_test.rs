//! B4a engine probe (`ENGINE_CHECKLIST.md`): `recipes::wilders_mixed_probe`
//! builds a synthetic, non-indicator recipe — a mean `Wilders(tr)` and a
//! composite `Wilders(tr)` sharing ONE `tr` source at ONE period, the exact
//! shape `classify_packed` buckets into a single lane group
//! (`(Kind::Wilders, tr)`). There is no hand oracle for this shape (no
//! indicator emits a bare composite Wilders row), so this file gates it
//! against an INDEPENDENTLY-WRITTEN reference recurrence built only from the
//! shared `tulip_rs::cse::multipliers::wilders_multiplier` utility — never
//! engine-vs-engine. Kept separate from `cse_kernel_registry_test.rs` for
//! that reason: it is a probe of the emitter's mixed-composite path, not an
//! indicator gate.
//!
//! Registered rows: `wilders_mixed_probe_n` (scalar) and
//! `wilders_mixed_probe_n_p` (packed) — `tulip_rs/src/cse/kernel/registry.rs`.

use tulip_rs::cse::kernel::registry::{wilders_mixed_probe_n, wilders_mixed_probe_n_p};
use tulip_rs::cse::*;

/// Independent reference TR, matching the recipe's own `tr_nodes()` shape:
/// `prev_close[t] = close[t-1]` for `t >= 1`, else `close[0]` (the "E6
/// bar-0 tr trick" — `Node::Lag` reads sample 0 for `t < n`, documented in
/// `tulip_rs/src/cse/kernel/warmup.rs`'s `K_LAG` arm). This is the same
/// formula the hand `tr`/`atr` indicators compute, just evaluated over the
/// FULL series (no leading bar dropped) to match the engine's own lead
/// bookkeeping.
fn reference_tr(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut out = vec![0.0; n];
    for t in 0..n {
        let prev_close = if t >= 1 { close[t - 1] } else { close[0] };
        let hl = high[t] - low[t];
        let hc = (high[t] - prev_close).abs();
        let lc = (low[t] - prev_close).abs();
        out[t] = hl.max(hc).max(lc);
    }
    out
}

/// Independent reference for BOTH the mean and composite Wilders rows over
/// one shared `tr` source, following `warmup.rs`'s own documented `K_WILDERS`
/// semantics exactly (seam at `feed = 0` since the recipe passes `feed:
/// false`): accumulate the raw sum for `t < period - 1`; at `t == period -
/// 1`, mean divides by `period`, composite keeps the raw sum; afterward both
/// step as `st = st.mul_add(mult, src * invms)` where `invms` is
/// `inv_multiplier` for mean and `1.0` for composite. Returns
/// `(mean_tail, composite_tail)`, each length `tr.len() - period` (the
/// `Sink::iface_lead(_, period)` convention), matching `out.rows[0]`/`[1]`.
fn reference_wilders_mixed(tr: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let (mult, inv_mult) = tulip_rs::cse::multipliers::wilders_multiplier(period);
    let mut mean_st = 0.0_f64;
    let mut comp_st = 0.0_f64;
    let mut mean_out = Vec::with_capacity(tr.len().saturating_sub(period));
    let mut comp_out = Vec::with_capacity(tr.len().saturating_sub(period));
    for (t, &s) in tr.iter().enumerate() {
        if t < period - 1 {
            mean_st += s;
            comp_st += s;
        } else if t == period - 1 {
            mean_st = (mean_st + s) / period as f64;
            comp_st += s;
            mean_out.push(mean_st);
            comp_out.push(comp_st);
        } else {
            mean_st = mean_st.mul_add(mult, s * inv_mult);
            comp_st = comp_st.mul_add(mult, s);
            mean_out.push(mean_st);
            comp_out.push(comp_st);
        }
    }
    // `Sink::iface_lead(_, period)` publishes from bar `period` onward, i.e.
    // drops the single `t == period - 1` seam bar that seeded the recurrence.
    mean_out.remove(0);
    comp_out.remove(0);
    (mean_out, comp_out)
}

fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
    (0..n).map(f).collect()
}
fn wiggle_hlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let close = series(n, |i| {
        100.0 + (i as f64 * 0.31).sin() * 6.0 + i as f64 * 0.01
    });
    let high = series(n, |i| close[i] + 1.0 + (i as f64 * 0.7).sin().abs());
    let low = series(n, |i| close[i] - 1.0 - (i as f64 * 0.53).cos().abs());
    (high, low, close)
}

fn assert_bits(got: &[f64], want: &[f64], label: &str) {
    assert_eq!(got.len(), want.len(), "{label}: length mismatch");
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(
            g.to_bits(),
            w.to_bits(),
            "{label}: bar {i} mismatch, got {g} want {w}"
        );
    }
}

#[test]
fn wilders_mixed_probe_bit_exact() {
    const P: [f64; 1] = [14.0];
    let period = P[0] as usize;
    let req = Request::new().add(recipes::wilders_mixed_probe(&P), None);
    for n in [60usize, 200, 700] {
        let (high, low, close) = wiggle_hlc(n);
        let streams = [high.as_slice(), low.as_slice(), close.as_slice()];
        let out = run_registered(&req, &streams).expect("wilders_mixed_probe registered");

        let tr = reference_tr(&high, &low, &close);
        let (mean_ref, comp_ref) = reference_wilders_mixed(&tr, period);
        assert_bits(&out.rows[0], &mean_ref, "mean row");
        assert_bits(&out.rows[1], &comp_ref, "composite row");
    }
}

#[test]
fn wilders_mixed_probe_scalar_packed_agree() {
    // Erase once, call BOTH generated functions directly on the same arena —
    // the `_n`/`_n_p` byte-equality check the checklist's B4a entry describes
    // ("scalar-vs-packed byte-equality on the same arena's two generated
    // functions"), not an engine-vs-engine value check.
    const P: [f64; 1] = [14.0];
    let req = Request::new().add(recipes::wilders_mixed_probe(&P), None);
    let (high, low, close) = wiggle_hlc(300);
    let streams = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (_a, _sig, vals, _) = erase_request(&req).unwrap();
    let out_scalar = wilders_mixed_probe_n(&vals, &streams);
    let out_packed = wilders_mixed_probe_n_p(&vals, &streams);
    assert_eq!(out_scalar.len(), out_packed.len(), "same number of rows");
    for (i, (r_s, r_p)) in out_scalar.iter().zip(out_packed.iter()).enumerate() {
        assert_bits(
            r_s.as_slice(),
            r_p.as_slice(),
            &format!("row {i}: packed vs scalar"),
        );
    }
}
