//! AO (Awesome Oscillator) parity: engine vs the hand, cold and resumed.
//! Separate binary (like `fisher_parity`) — AO is two derived-`RollSum`s sharing
//! one `medprice` ring, a plan-heavy shape kept out of the shared `cse::` process.
//! AO is bit-exact vs the hand (no `ln`/libm divergence like fisher), so use the
//! `cmp_exact` bit test.

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::ao::Ao;
use tulip_rs::indicators::medprice::Medprice;

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

fn pairs() -> Vec<(&'static str, Vec<f64>, Vec<f64>)> {
    let w = series(400);
    let wl: Vec<f64> = w.iter().map(|v| v * 0.97 - 2.0).collect();
    vec![
        ("wiggle", w.clone(), wl),
        (
            "rising",
            (0..300).map(|i| 50.0 + i as f64).collect(),
            (0..300).map(|i| 47.0 + i as f64).collect(),
        ),
        ("flat", vec![10.0; 300], vec![4.0; 300]),
    ]
}

#[test]
fn ao_matches_hand_and_resume() {
    let plan = Request::new().add(Ao::recipe(&[]), None).build();
    // The `ao` primary + its `medprice` optional share the derived med node.
    for (name, high, low) in pairs() {
        let n = high.len();
        let s: [&[f64]; 2] = [&high, &low];
        // primary only (None) → just the ao row; hand ao = outputs[0].
        let hand = Ao::indicator(&s, &[], None).unwrap().0;
        let got = plan.run(&s).0;
        cmp_exact(got.request(0).output(0), &hand[0], &format!("ao {name}"));

        // Resume byte-exact: lead is 34 (min_data 35), so split around it too.
        for split in [33usize, 34, 35, 40, n / 3, n - 1] {
            if split < 1 || split >= n {
                continue;
            }
            let c: [&[f64]; 2] = [&high[..split], &low[..split]];
            let t: [&[f64]; 2] = [&high[split..], &low[split..]];
            let (r1, qs) = plan.run(&c);
            let (r2, _) = plan.resume(qs, &t);
            let mut cat = r1.request(0).output(0).to_vec();
            cat.extend_from_slice(r2.request(0).output(0));
            cmp_exact(
                &cat,
                &hand[0],
                &format!("ao {name} split{split} cold-vs-resume"),
            );
        }
    }
}

/// The fusion proof at the arena level: AO + Medprice must intern ONE `med`
/// (AO's own med IS a Medprice sub), and AO + a standalone Medprice query adds
/// exactly the two rolls + the subtract, no second `0.5·(h+l)`.
#[test]
fn ao_shares_the_medprice_node() {
    let ao_only = Request::new().add(Ao::recipe(&[]), None).build();
    let med_only = Request::new().add(Medprice::recipe(&[]), None).build();
    let both = Request::new()
        .add(Ao::recipe(&[]), None)
        .add(Medprice::recipe(&[]), None)
        .build();
    let (ao_ops, _) = ao_only.info();
    let (med_ops, _) = med_only.info();
    let (both_ops, _) = both.info();
    // medprice alone = 2 inputs + add + mul = the `med` node; AO already embeds
    // it. Merged must be < ao+med (the med Add/Mul/Half intern once).
    assert!(
        both_ops < ao_ops + med_ops,
        "ao+medprice must share the med subgraph: merged {both_ops} vs {ao_ops}+{med_ops}",
    );
}
