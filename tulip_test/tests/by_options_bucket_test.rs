//! Hand-SIMD contract pin, found while building the fused bench matrix
//! (2026-10-12): `indicator_by_options::<N>` is only correct at the
//! road_train BUCKET widths (8/4/2). A probe of `::<6>`/`::<3>` computed
//! rows whose values mismatched the hand at essentially every element
//! (lane misalignment) — silently. The bench rivals therefore compose
//! bucket calls (4+2) for six-set shapes. If this test's broken-width
//! companion is ever added after a road_train fix, revisit the benches.
//!
//! Here we pin the VALID widths against the scalar hand, so a future
//! refactor cannot quietly regress them too.
use tulip_rs::indicator_types::{Indicator, IndicatorByOptions};
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::sma::Sma;

fn series() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let close: Vec<f64> = (0..2000)
        .map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
        .collect();
    let high: Vec<f64> = close.iter().map(|v| v + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 0.5).collect();
    (high, low, close)
}

#[test]
fn sma_by_options_width4_and_2_match_hand() {
    let (_, _, close) = series();
    let one: [&[f64]; 1] = [close.as_slice()];
    let o4: [[f64; 1]; 4] = [[3.0], [7.0], [14.0], [20.0]];
    let orefs4: [&[f64; 1]; 4] = [&o4[0], &o4[1], &o4[2], &o4[3]];
    let (rows, _) = Sma::indicator_by_options::<4>(&one, &orefs4, None).unwrap();
    for (li, &p) in [3.0, 7.0, 14.0, 20.0].iter().enumerate() {
        let (hand, _) = Sma::indicator(&one, &[p], None).unwrap();
        assert_eq!(rows[li][0].len(), hand[0].len(), "lane {li} len");
        for (i, (a, b)) in rows[li][0].iter().zip(hand[0].iter()).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "sma N=4 lane {li} bar {i}");
        }
    }
    let o2: [[f64; 1]; 2] = [[50.0], [100.0]];
    let orefs2: [&[f64; 1]; 2] = [&o2[0], &o2[1]];
    let (rows, _) = Sma::indicator_by_options::<2>(&one, &orefs2, None).unwrap();
    for (li, &p) in [50.0, 100.0].iter().enumerate() {
        let (hand, _) = Sma::indicator(&one, &[p], None).unwrap();
        for (i, (a, b)) in rows[li][0].iter().zip(hand[0].iter()).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "sma N=2 lane {li} bar {i}");
        }
    }
}

#[test]
fn atr_by_options_width4_matches_hand() {
    let (high, low, close) = series();
    let three: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
    let o: [[f64; 1]; 4] = [[9.0], [14.0], [21.0], [28.0]];
    let orefs: [&[f64; 1]; 4] = [&o[0], &o[1], &o[2], &o[3]];
    let (rows, _) = Atr::indicator_by_options::<4>(&three, &orefs, None).unwrap();
    for (li, &p) in [9.0, 14.0, 21.0, 28.0].iter().enumerate() {
        let (hand, _) = Atr::indicator(&three, &[p], None).unwrap();
        assert_eq!(rows[li][0].len(), hand[0].len(), "atr lane {li} len");
        for (i, (a, b)) in rows[li][0].iter().zip(hand[0].iter()).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "atr N=4 lane {li} bar {i}");
        }
    }
}
