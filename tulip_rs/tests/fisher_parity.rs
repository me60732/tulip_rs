//! E15 `fisher` parity for the CSE/JIT engine against the hand scalar.
//!
//! A SEPARATE integration binary (like `minmax_consumers`): `fisher` is the first
//! plan to consume `Iir`/`IirClamped`/`Prev` + a derived roll + a slot read at once,
//! so it belongs out of the shared `cse::` lib process.
//!
//! Two distinct tolerance gates:
//! · engine vs the HAND scalar is **1e-12 RELATIVE** — `Node::Ln` is
//!   `math_simd::ln_unchecked`, not libm, and the hand's OWN `fisher_test.rs`
//!   compares its scalar/SIMD paths at `EPSILION = 1e-12` (the hand is not
//!   bit-exact with itself here — Handover §6.35).
//! · engine COLD vs engine RESUMED is **byte-exact** — same compiled loop, so the
//!   `ln` approximation cancels; a stitch divergence is a real bug.

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::fisher::Fisher;

fn series(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
        .collect()
}

/// 1e-12 relative with an absolute floor (a near-zero value isn't held tighter than
/// its magnitude allows).
fn cmp_rel(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
    for i in 0..a.len() {
        let scale = a[i].abs().max(1.0);
        assert!(
            (a[i] - b[i]).abs() <= 1e-12 * scale,
            "{what}: mismatch at {i}: {:?} vs {:?}",
            a[i],
            b[i]
        );
    }
}

fn cmp_bits(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
    for i in 0..a.len() {
        assert_eq!(
            a[i].to_bits(),
            b[i].to_bits(),
            "{what}: byte mismatch at {i}: {:?} vs {:?}",
            a[i],
            b[i]
        );
    }
}

/// `(high, low)` pairs. `flat` (all high=10, low=4) makes every window's `med`
/// the constant 7 ⇒ `max−min==0` ⇒ the `mm` ε floor fires every bar — where a
/// wrong derived-roll cold seed (§6.36) and the clamp's fixed point show up.
fn hlc_pairs() -> Vec<(&'static str, Vec<f64>, Vec<f64>)> {
    let wiggle = series(400);
    let wlow: Vec<f64> = wiggle.iter().map(|v| v * 0.97 - 2.0).collect();
    vec![
        ("wiggle", wiggle.clone(), wlow),
        (
            "rising",
            (0..300).map(|i| 50.0 + i as f64).collect(),
            (0..300).map(|i| 47.0 + i as f64).collect(),
        ),
        (
            "ties",
            (0..300).map(|i| ((i * 7) % 11) as f64).collect(),
            (0..300).map(|i| ((i * 5) % 13) as f64).collect(),
        ),
        ("flat", vec![10.0; 300], vec![4.0; 300]),
    ]
}

#[test]
fn fisher_matches_hand_1e12_and_resume_is_byte_exact() {
    for p in [2u32, 5, 9, 14, 20] {
        let opt = [p as f64];
        let plan = Request::new().add(Fisher::recipe(&opt), None).build();
        let (nops, nsinks) = plan.info();
        assert!(nops >= 2, "fisher p{p}: degenerate plan ({nops} ops)");
        assert_eq!(nsinks, 2, "fisher p{p}: fisher + signal");

        for (name, high, low) in hlc_pairs() {
            let n = high.len();
            let s: [&[f64]; 2] = [&high, &low];
            let hand = Fisher::indicator(&s, &opt, None).unwrap().0;
            let got = plan.run(&s).0;
            for (row, label) in [(0usize, "fisher"), (1, "signal")] {
                cmp_rel(
                    got.request(0).output(row),
                    &hand[row],
                    &format!("fisher {name} p{p} {label} vs hand"),
                );
            }

            // Cold-vs-resume byte-exact, straddling the first published bar.
            let lead = p as usize - 1;
            for split in [lead - 1, lead, lead + 1, p as usize, n / 3, n - 1] {
                if split < 1 || split >= n {
                    continue;
                }
                let c: [&[f64]; 2] = [&high[..split], &low[..split]];
                let t: [&[f64]; 2] = [&high[split..], &low[split..]];
                let (r1, qs) = plan.run(&c);
                let (r2, _) = plan.resume(qs, &t);
                for (row, label) in [(0usize, "fisher"), (1, "signal")] {
                    let mut cat = r1.request(0).output(row).to_vec();
                    cat.extend_from_slice(r2.request(0).output(row));
                    cmp_bits(
                        &cat,
                        got.request(0).output(row),
                        &format!("fisher {name} p{p} {label} split{split} cold-vs-resume"),
                    );
                }
            }
        }
    }
}
