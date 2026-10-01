//! E9 M2 parity for the range-based roll consumers (WILLR, DONCHIAN) — the
//! `(period, period − 1)` window family, run in its OWN process.
//!
//! Why a separate binary: the `cse::` lib suite SIGSEGVs intermittently once
//! these matrices live in it (6/6 with the `cse::` filter, 0/6 under
//! `--nocapture`, and the full lib run passes) — the pattern of a
//! layout-sensitive fault in the shared cranelift executable arena, the same
//! class already documented for this harness. Plan-heavy roll sweeps therefore
//! go in their own process, like `differential_parity`, `jit_alloc_count` and
//! `reg_pressure_probe`. This is CONTAINMENT, not a root cause: the walk's
//! addresses audited in-bounds against the real backing spans, so the fault is
//! not a modelled range violation and stays open.
//!
//! Coverage is not reduced: every dataset × period × split assertion that lived
//! in the lib test is here, and the regime sweep (7 shapes × 3 pseudo-random
//! periods, cold AND streamed) additionally runs in `differential_parity`.

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::ichimoku::Ichimoku;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::willr::Willr;

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

/// `(high, low)` pairs: a wiggly series, a strictly rising one (the rolling min
/// is then always stale → rescan nearly every bar), a tie-heavy one (decides
/// `trail` by recency) and a flat one.
fn hlc_pairs() -> Vec<(&'static str, Vec<f64>, Vec<f64>)> {
    let wiggle = series(400);
    let wlow: Vec<f64> = wiggle.iter().map(|v| v * 0.97 - 2.0).collect();
    vec![
        ("wiggle", wiggle.clone(), wlow),
        (
            "rising",
            (0..300).map(|i| i as f64).collect(),
            (0..300).map(|i| i as f64 - 1.0).collect(),
        ),
        (
            "ties",
            (0..300).map(|i| ((i * 7) % 11) as f64).collect(),
            (0..300).map(|i| ((i * 5) % 13) as f64).collect(),
        ),
        ("flat", vec![10.0; 300], vec![4.0; 300]),
    ]
}

/// Adds `close` for willr. `flat-range` is the degenerate case the hand's guard
/// exists for: max == min exactly, so `dif = 0` → the division yields ±inf and
/// the `WhereGe` select must discard it for `0.0` on EVERY bar.
fn datasets() -> Vec<(&'static str, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let mut out = Vec::new();
    for (shape, high, low) in hlc_pairs() {
        let close: Vec<f64> = (0..high.len())
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        out.push((shape, high, low, close));
    }
    out.push((
        "flat-range",
        vec![10.0; 300],
        vec![10.0; 300],
        vec![12.0; 300],
    ));
    out
}

/// WILLR — `RollMax(high)` / `RollMin(low)` at the `(p, p−1)` window plus the
/// degenerate-range guard. Bit-exact on the primary AND both mirrors, including
/// their DIFFERENT lengths (the mirrors start one bar earlier — hand
/// `init_state` stores them at `look_back`, then `slice_outputs_start!` offsets
/// the rest). Plan built once per period, not per dataset.
#[test]
fn willr_matches_hand() {
    for period in [7usize, 14, 25] {
        let opt = [period as f64];
        let plan = Request::new().add_all(Willr::recipe(&opt)).build();
        for (shape, high, low, close) in datasets() {
            let n = high.len();
            let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
            let hand = Willr::indicator(&s, &opt, Some(&[true, true])).unwrap().0;
            let (res, _) = plan.run(&s);
            cmp_exact(
                res.request(0).output(0),
                &hand[0],
                &format!("willr {shape} p{period}"),
            );
            cmp_exact(
                res.optional[0][0].as_slice(),
                &hand[1],
                &format!("willr {shape} p{period} min mirror"),
            );
            cmp_exact(
                res.optional[0][1].as_slice(),
                &hand[2],
                &format!("willr {shape} p{period} max mirror"),
            );
            assert_eq!(hand[0].len(), n - period, "primary starts at bar p");
            assert_eq!(hand[1].len(), n - period + 1, "mirrors start at p−1");

            for split in [period - 1, period, period + 1, n / 3] {
                let c: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let t: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (r1, qs) = plan.run(&c);
                let (r2, _) = plan.resume(qs, &t);
                let mut a = r1.request(0).output(0).to_vec();
                a.extend_from_slice(r2.request(0).output(0));
                cmp_exact(
                    &a,
                    &hand[0],
                    &format!("willr {shape} p{period} split{split}"),
                );
                for (i, want) in [(0usize, &hand[1]), (1, &hand[2])] {
                    let mut m = r1.optional[0][i].to_vec();
                    m.extend_from_slice(r2.optional[0][i].as_slice());
                    cmp_exact(
                        &m,
                        want,
                        &format!("willr {shape} p{period} split{split} mirror{i}"),
                    );
                }
            }
        }
    }
}

/// DONCHIAN — `(lower, middle, upper)` where `middle = 0.5·(max+min)` of the
/// two rolls. All three rows start at bar `period − 1` (the hand overrides
/// `min_data = period`). Also the shape whose plan count made the shared lib
/// suite fault, so it is asserted bit-exact here at every split too.
#[test]
fn donchian_matches_hand() {
    for period in [7usize, 14, 25] {
        let opt = [period as f64];
        let plan = Request::new()
            .add(DonchianChannel::recipe(&opt), None)
            .build();
        for (shape, high, low, _close) in datasets() {
            let n = high.len();
            let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
            let hand = DonchianChannel::indicator(&s, &opt, None).unwrap().0;
            let (res, _) = plan.run(&s);
            for (row, want) in [(0usize, &hand[0]), (1, &hand[1]), (2, &hand[2])] {
                cmp_exact(
                    res.request(0).output(row),
                    want,
                    &format!("donchian {shape} p{period} row{row}"),
                );
            }
            assert_eq!(hand[0].len(), n - period + 1, "rows start at p−1");

            for split in [period - 1, period, period + 1, n / 3] {
                let c: [&[f64]; 2] = [&high[..split], &low[..split]];
                let t: [&[f64]; 2] = [&high[split..], &low[split..]];
                let (r1, qs) = plan.run(&c);
                let (r2, _) = plan.resume(qs, &t);
                for (row, want) in [(0usize, &hand[0]), (1, &hand[1]), (2, &hand[2])] {
                    let mut cat = r1.request(0).output(row).to_vec();
                    cat.extend_from_slice(r2.request(0).output(row));
                    cmp_exact(
                        &cat,
                        want,
                        &format!("donchian {shape} p{period} split{split} row{row}"),
                    );
                }
            }
        }
    }
}

/// E9 SEED LINK — the nested-window case: `donchian(20)` advances
/// `RollMax{high, p=20, lb=19}`, and a wider `max(50)` over the SAME column has
/// `lb=49`, so its rescan walks only the 30-bar gap beyond donchian's window,
/// seeded by donchian's value — and inherits donchian's `trail` whenever the gap
/// fails to STRICTLY beat it. Asserted by COUNTING the links (values alone
/// can't show a missing link), then bit-exact against both hand indicators, cold
/// and streamed across the guard seams.
///
/// The data targets the one shape that can break a seeded walk: `9.0` spikes 40
/// bars apart, so inside a 50-bar window the SAME maximum sits in the gap AND in
/// the narrow window at once. The newest occurrence must win — exactly what the
/// strict `>` protects, and what a `>=` would get wrong in `aroon`, whose output
/// IS the trail.
#[test]
fn nested_rolls_seed_the_wide_walk() {
    let n = 500usize;
    let hi: Vec<f64> = (0..n)
        .map(|i| if i % 40 == 0 { 9.0 } else { 5.0 })
        .collect();
    let lo: Vec<f64> = (0..n)
        .map(|i| if i % 40 == 0 { 4.0 } else { 1.0 })
        .collect();
    let s: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
    let d20 = [20.0f64];
    let m50 = [50.0f64];

    let plan = Request::new()
        .add(DonchianChannel::recipe(&d20), None)
        .add(Max::recipe(&m50), None)
        .build();
    assert_eq!(
        plan.roll_seeds().len(),
        1,
        "expected exactly one link (wide max(50) ← narrow donchian max), got {:?}",
        plan.roll_seeds()
    );

    let dh = DonchianChannel::indicator(&s, &d20, None).unwrap().0;
    let mh = Max::indicator(&[hi.as_slice()], &m50, None).unwrap().0;
    let (res, _) = plan.run(&s);
    for (row, want) in [(0usize, &dh[0]), (1, &dh[1]), (2, &dh[2])] {
        cmp_exact(
            res.request(0).output(row),
            want,
            &format!("seeded donchian row{row}"),
        );
    }
    cmp_exact(res.request(1).output(0), &mh[0], "seeded max(50)");

    for split in [20usize, 50, 51, 250, n - 1] {
        let c: [&[f64]; 2] = [&hi[..split], &lo[..split]];
        let t: [&[f64]; 2] = [&hi[split..], &lo[split..]];
        let (r1, st) = plan.run(&c);
        let (r2, _) = plan.resume(st, &t);
        for (req, row, want) in [
            (0usize, 0usize, &dh[0]),
            (0, 1, &dh[1]),
            (0, 2, &dh[2]),
            (1, 0, &mh[0]),
        ] {
            let mut cat = r1.request(req).output(row).to_vec();
            cat.extend_from_slice(r2.request(req).output(row));
            cmp_exact(&cat, want, &format!("seeded split{split} r{req}o{row}"));
        }
    }

    // The emitter runs a TWO-PHASE order (all rolls first, narrow→wide), so the
    // link forms no matter how the caller listed the requests — pinned here
    // because a regression to arena-order emission would silently lose sharing.
    let rev = Request::new()
        .add(Max::recipe(&m50), None)
        .add(DonchianChannel::recipe(&d20), None)
        .build();
    assert_eq!(
        rev.roll_seeds().len(),
        1,
        "wide-first must link just as well (emission is two-phase)"
    );
    let rres = rev.run(&s).0;
    cmp_exact(rres.request(0).output(0), &mh[0], "fallback max(50)");
    cmp_exact(
        rres.request(1).output(1),
        &dh[1],
        "fallback donchian middle",
    );
}

/// E9: the links **chain**, which is the sharing this mechanism exists for. Three
/// windows over the one `high` column — `donchian(20)` (lb 19), `max(50)` (lb 49),
/// `max(100)` (lb 99) — must form TWO links (50←19, 100←49), so the rescan work on
/// that column is the union of the gaps (19 + 30 + 50) rather than three
/// independent walks (19 + 50 + 100). Interning can NEVER give this: different
/// periods are different nodes by definition, so the nested-gap chain is the only
/// way they cooperate. Bit-exact against all three hand indicators, cold and
/// streamed, on data that keeps the tie-in-gap shape while a slow drift puts the
/// middle window's champion inside the narrow window (where a chain must INHERIT
/// the seed's trail, not rediscover it).
#[test]
fn seed_links_chain_across_three_windows() {
    let n = 600usize;
    let hi: Vec<f64> = (0..n)
        .map(|i| if i % 40 == 0 { 9.0 } else { 5.0 } + (i as f64) * 0.0005)
        .collect();
    let lo: Vec<f64> = hi.iter().map(|v| v - 1.5).collect();
    let s: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];

    let plan = Request::new()
        .add(DonchianChannel::recipe(&[20.0]), None)
        .add(Max::recipe(&[50.0]), None)
        .add(Max::recipe(&[100.0]), None)
        .build();
    assert_eq!(
        plan.roll_seeds().len(),
        2,
        "the chain must form both links, got {:?}",
        plan.roll_seeds()
    );

    let d = DonchianChannel::indicator(&s, &[20.0], None).unwrap().0;
    let m50 = Max::indicator(&[hi.as_slice()], &[50.0], None).unwrap().0;
    let m100 = Max::indicator(&[hi.as_slice()], &[100.0], None).unwrap().0;
    let (res, _) = plan.run(&s);
    cmp_exact(res.request(0).output(2), &d[2], "chain: donchian upper");
    cmp_exact(res.request(0).output(1), &d[1], "chain: donchian middle");
    cmp_exact(res.request(1).output(0), &m50[0], "chain: max(50)");
    cmp_exact(res.request(2).output(0), &m100[0], "chain: max(100)");

    for split in [50usize, 100, 101, 300, n - 1] {
        let c: [&[f64]; 2] = [&hi[..split], &lo[..split]];
        let t: [&[f64]; 2] = [&hi[split..], &lo[split..]];
        let (r1, st) = plan.run(&c);
        let (r2, _) = plan.resume(st, &t);
        for (req, row, want) in [
            (0usize, 1usize, &d[1]),
            (0, 2, &d[2]),
            (1, 0, &m50[0]),
            (2, 0, &m100[0]),
        ] {
            let mut cat = r1.request(req).output(row).to_vec();
            cat.extend_from_slice(r2.request(req).output(row));
            cmp_exact(&cat, want, &format!("chain split{split} r{req}o{row}"));
        }
    }
}

/// E9: link formation is **order-independent**. The emitter walks in two phases
/// (all rolls first, sorted by stream/direction/ascending `look_back`), so the
/// same three windows chain identically whether listed narrow-first or
/// wide-first — the sharing cannot depend on how a user happened to order the
/// query. This also guards the phase split itself: a roll reads only its stream
/// and its own carried state, so hoisting rolls ahead of every other op is safe.
#[test]
fn seed_links_are_order_independent() {
    let n = 600usize;
    let hi: Vec<f64> = (0..n).map(|i| (i % 40) as f64).collect();
    let lo: Vec<f64> = hi.iter().map(|v| v - 1.0).collect();
    let s: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
    let d20 = [20.0f64];
    let m50 = [50.0f64];
    let m100 = [100.0f64];
    let narrow_first = Request::new()
        .add(DonchianChannel::recipe(&d20), None)
        .add(Max::recipe(&m50), None)
        .add(Max::recipe(&m100), None)
        .build();
    let wide_first = Request::new()
        .add(Max::recipe(&m100), None)
        .add(Max::recipe(&m50), None)
        .add(DonchianChannel::recipe(&d20), None)
        .build();
    assert_eq!(
        wide_first.roll_seeds().len(),
        2,
        "both orders must chain twice, got {:?}",
        wide_first.roll_seeds()
    );
    assert_eq!(
        narrow_first.roll_seeds().len(),
        wide_first.roll_seeds().len(),
        "link count must not depend on request order"
    );

    let m100 = Max::indicator(&[hi.as_slice()], &m100, None).unwrap().0;
    let m50 = Max::indicator(&[hi.as_slice()], &m50, None).unwrap().0;
    let d = DonchianChannel::indicator(&s, &d20, None).unwrap().0;
    let (res, _) = wide_first.run(&s);
    cmp_exact(res.request(0).output(0), &m100[0], "wide-first max(100)");
    cmp_exact(res.request(1).output(0), &m50[0], "wide-first max(50)");
    for (row, want) in [(0usize, &d[0]), (1, &d[1]), (2, &d[2])] {
        cmp_exact(
            res.request(2).output(row),
            want,
            &format!("wide-first donchian row{row}"),
        );
    }
}

/// E9 M4: ICHIMOKU — the densest nesting case, and the one that pays for the
/// chain rather than merely tolerating it. Six rolls (short / medium / ultra ×
/// max / min), three strictly increasing windows per column, so FOUR links form
/// and each column's rescan work becomes the union of the gaps: for 9/26 that is
/// 9 + 17 + 26 loads instead of 9 + 26 + 52.
///
/// Bit-exact on all FOUR lines against the hand, cold and streamed — which also
/// pins the three DISTINCT publish leads the hand's `slot_lengths` imply
/// (conversion from `s−1`, base from `l−1`, both spans from `2l−1`), including
/// the point where the hand withholds `span_a` until `2l−1` even though it is
/// computable from `l−1`: a sink lead, not a padding feature. `close` is
/// deliberately NOT read (the engine returns no `lagging_span`), so the plan
/// takes 2 streams where the hand takes 3.
#[test]
fn ichimoku_matches_hand() {
    let (shape, hi, lo) = hlc_pairs().into_iter().next().unwrap();
    let n = hi.len();
    let cl: Vec<f64> = (0..n).map(|i| lo[i] + (hi[i] - lo[i]) * 0.5).collect();
    // validate_options needs 1 ≤ short < long; ultra = 2·long is the binding
    // warm-up, so keep 2·long well inside the series.
    for (short, long) in [(9usize, 26usize), (7, 14), (14, 55)] {
        let opt = [short as f64, long as f64];
        let plan = Request::new().add(Ichimoku::recipe(&opt), None).build();
        assert_eq!(
            plan.roll_seeds().len(),
            4,
            "{short}/{long}: two links per column (medium←short, ultra←medium)"
        );
        assert_eq!(plan.n_streams(), 2, "ichimoku's math never reads close");

        let s: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
        let hand = Ichimoku::indicator(&s, &opt, None).unwrap().0;
        let (res, _) = plan.run(&[hi.as_slice(), lo.as_slice()]);
        // The hand's four lines start at three DIFFERENT bars.
        assert_eq!(hand[0].len(), n - short + 1);
        assert_eq!(hand[1].len(), n - long + 1);
        assert_eq!(hand[2].len(), n - 2 * long + 1);
        assert_eq!(hand[3].len(), n - 2 * long + 1);
        for row in 0..4 {
            cmp_exact(
                res.request(0).output(row),
                &hand[row],
                &format!("ichimoku {shape} {short}/{long} row{row}"),
            );
        }

        let ultra_lead = 2 * long - 1;
        for split in [short - 1, long - 1, long, ultra_lead, ultra_lead + 1, n / 3] {
            let c2: [&[f64]; 2] = [&hi[..split], &lo[..split]];
            let t2: [&[f64]; 2] = [&hi[split..], &lo[split..]];
            let (r1, st) = plan.run(&c2);
            let (r2, _) = plan.resume(st, &t2);
            // Chunked engine rows must TILE the single cold run's row — each line
            // from its own lead, so a piece may legitimately be empty. The hand
            // can't be called on a short first chunk here (its `min_data` is
            // `3·long`), so the hand comparison is the cold run above and this
            // asserts the cold == resumed invariant itself.
            for row in 0..4 {
                let mut cat = r1.request(0).output(row).to_vec();
                cat.extend_from_slice(r2.request(0).output(row));
                cmp_exact(
                    &cat,
                    &hand[row],
                    &format!("ichimoku {short}/{long} split{split} row{row}"),
                );
            }
        }
    }
}
