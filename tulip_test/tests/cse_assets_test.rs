//! Asset-lane kernel parity (M2): `<name>_assets::<L>` must reproduce,
//! byte-for-byte, the per-asset scalar `run_registered` result for every
//! registered pattern. The lane axis may change only SPEED, never values —
//! these tests are the correctness gate for the whole asset emitter in
//! `tulip_rs_macros::cse::codegen::run_assets_fn`.

use tulip_rs::cse::*;
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::aroon::Aroon;
use tulip_rs::indicators::aroonosc::AroonOsc;
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::bbands::BBands;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::ichimoku::Ichimoku;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::min::Min;
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::ppo::Ppo;
use tulip_rs::indicators::sma::Sma;
use tulip_rs::indicators::stddev::StdDev;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_rs::indicators::willr::Willr;
use tulip_rs::indicators::wma::Wma;

fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
    (0..n).map(f).collect()
}

/// Four distinct shapes per slot kind — phase-shifted sines, plateau ties
/// (the rolls' rescan torture), and a monotone ramp. Same length: the arena
/// is sized from the streams and must be equal per asset.
const N: usize = 660;

fn price(k: usize) -> Vec<f64> {
    match k {
        0 => series(N, |i| 100.0 + (i as f64 * 0.013).sin() * 7.0),
        1 => series(N, |i| 90.0 + (i as f64 * 0.029 + 1.7).sin() * 4.0),
        2 => series(N, |i| 100.0 + ((i % 9) / 2) as f64),
        _ => series(N, |i| 80.0 + i as f64 * 0.03),
    }
}

fn hi(k: usize) -> Vec<f64> {
    price(k).iter().map(|v| v + 1.0 + 0.25 * k as f64).collect()
}
fn lo(k: usize) -> Vec<f64> {
    price(k).iter().map(|v| v - 0.5 - 0.4 * k as f64).collect()
}

/// The lane-vs-scalar gate: one plan, L assets, every lane byte-identical to
/// the standalone scalar run of the same request on that asset's streams.
#[allow(clippy::type_complexity)]
fn lanes_match_scalar<const L: usize>(req: &Request, assets: &[&[&[f64]]; L], label: &str) {
    let out = run_assets_registered(req, assets).unwrap_or_else(|| panic!("{label}: no lane hit"));
    assert_eq!(out.per_asset.len(), L, "{label}: lane count");
    for a in 0..L {
        let want = run_registered(req, assets[a]).unwrap_or_else(|| panic!("{label}: scalar miss"));
        let got: Vec<&[f64]> = out.per_asset[a].rows.iter().map(|r| r.as_slice()).collect();
        let exp: Vec<&[f64]> = want.rows.iter().map(|r| r.as_slice()).collect();
        if got != exp {
            for (r, (g, e)) in got.iter().zip(exp.iter()).enumerate() {
                if g.len() != e.len() {
                    panic!("{label} asset {a} row {r}: len {} != {}", g.len(), e.len());
                }
                if let Some(d) = g.iter().zip(e.iter()).position(|(x, y)| x != y) {
                    panic!(
                        "{label} asset {a} row {r} first diff at idx {d}: lane {:?} vs scalar {:?}",
                        g[d], e[d]
                    );
                }
            }
            unreachable!();
        }
    }
}

/// Four price series pre-bound (single-stream patterns).
fn prices() -> [Vec<f64>; 4] {
    [price(0), price(1), price(2), price(3)]
}

// ── ema family ────────────────────────────────────────────────────────────

#[test]
fn lanes_apo_ppo_macd() {
    let req = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Ppo::recipe(&[12.0, 26.0]), None)
        .add(Macd::recipe(&[12.0, 26.0, 9.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "trio");
    let b: [&[&[f64]]; 2] = [&a0, &a3];
    lanes_match_scalar(&req, &b, "trio L2");
}

#[test]
fn lanes_ema_six() {
    let req = Request::new()
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
        .add(Apo::recipe(&[10.0, 20.0]), None)
        .add(Ppo::recipe(&[6.0, 14.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "ema_six");
}

#[test]
fn lanes_ema_chains() {
    let mut req = Request::new();
    for q in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
        req = req.add(Ema::recipe(&[q]), None);
    }
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "ema_chains");
}

// ── atr family ────────────────────────────────────────────────────────────

#[test]
fn lanes_atr_natr() {
    let req = Request::new()
        .add(Atr::recipe(&[14.0]), None)
        .add(Natr::recipe(&[14.0]), None);
    let (h0, h3) = (hi(0), hi(3));
    let (l1, l2) = (lo(1), lo(2));
    let c = price(2);
    let s = price(0);
    let a0 = [h0.as_slice(), l1.as_slice(), c.as_slice()];
    let a1 = [l1.as_slice(), h0.as_slice(), c.as_slice()];
    let a2 = [h3.as_slice(), l2.as_slice(), s.as_slice()];
    let a3 = [c.as_slice(), c.as_slice(), c.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "atr_natr");
    let b: [&[&[f64]]; 2] = [&a1, &a2];
    lanes_match_scalar(&req, &b, "atr_natr L2");
}

#[test]
fn lanes_natr_supertrend() {
    let req = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let (h1, h2) = (hi(1), hi(2));
    let (l0, l3) = (lo(0), lo(3));
    let p0 = price(0);
    let p2 = price(2);
    let a0 = [h1.as_slice(), l3.as_slice(), p0.as_slice()];
    let a1 = [p0.as_slice(), p0.as_slice(), p0.as_slice()];
    let a2 = [l3.as_slice(), h1.as_slice(), p2.as_slice()];
    let a3 = [h2.as_slice(), l0.as_slice(), l3.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "natr_st");
}

#[test]
fn lanes_atr_multi() {
    let req = Request::new()
        .add(Atr::recipe(&[9.0]), None)
        .add(Atr::recipe(&[14.0]), None)
        .add(Atr::recipe(&[21.0]), None);
    let (h0, h1) = (hi(0), hi(1));
    let (l1, l2) = (lo(1), lo(2));
    let c = price(3);
    let p0 = price(0);
    let p1 = price(1);
    let p2 = price(2);
    let a0 = [h0.as_slice(), l2.as_slice(), c.as_slice()];
    let a1 = [c.as_slice(), c.as_slice(), p0.as_slice()];
    let a2 = [h1.as_slice(), l1.as_slice(), l1.as_slice()];
    let a3 = [p2.as_slice(), p1.as_slice(), c.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "atr_multi");
}

// ── rolling sums ──────────────────────────────────────────────────────────

#[test]
fn lanes_sma_stddev() {
    let req = Request::new()
        .add(Sma::recipe(&[14.0]), None)
        .add(StdDev::recipe(&[14.0]), Some(&[true]));
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "sma_stddev");
}

#[test]
fn lanes_sma_wma() {
    let req = Request::new()
        .add(Sma::recipe(&[14.0]), None)
        .add(Wma::recipe(&[14.0]), Some(&[true]));
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "sma_wma");
}

#[test]
fn lanes_bbands_wma() {
    let req = Request::new()
        .add(BBands::recipe(&[20.0, 2.0]), None)
        .add(Wma::recipe(&[20.0]), Some(&[true]));
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "bbands_wma");
    let b: [&[&[f64]]; 2] = [&a2, &a0];
    lanes_match_scalar(&req, &b, "bbands_wma L2");
}

#[test]
fn lanes_sma_six() {
    let mut req = Request::new();
    for q in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
        req = req.add(Sma::recipe(&[q]), None);
    }
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "sma_six");
}

// ── trail-rescan hub ──────────────────────────────────────────────────────

#[test]
fn lanes_max_min() {
    let req = Request::new()
        .add(Max::recipe(&[14.0]), None)
        .add(Min::recipe(&[14.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "max_min");
    let b: [&[&[f64]]; 2] = [&a1, &a2];
    lanes_match_scalar(&req, &b, "max_min L2");
}

#[test]
fn lanes_aroon_osc() {
    let req = Request::new()
        .add(Aroon::recipe(&[14.0]), None)
        .add(AroonOsc::recipe(&[14.0]), Some(&[true, true]));
    let h2 = hi(2);
    let (l1, l3) = (lo(1), lo(3));
    let p0 = price(0);
    let p1 = price(1);
    let p2 = price(2);
    let p3 = price(3);
    let a0 = [h2.as_slice(), l1.as_slice()];
    let a1 = [p0.as_slice(), p3.as_slice()];
    let a2 = [l3.as_slice(), p1.as_slice()];
    let a3 = [p2.as_slice(), p2.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "aroon_osc");
}

#[test]
fn lanes_willr_donch() {
    let req = Request::new()
        .add(Willr::recipe(&[14.0]), Some(&[true, true]))
        .add(DonchianChannel::recipe(&[14.0]), None);
    let (h1, h3) = (hi(1), hi(3));
    let (l0, l2) = (lo(0), lo(2));
    let p0 = price(0);
    let p1 = price(1);
    let p2 = price(2);
    let p3 = price(3);
    let a0 = [h1.as_slice(), l2.as_slice(), p0.as_slice()];
    let a1 = [p0.as_slice(), p0.as_slice(), p2.as_slice()];
    let a2 = [h3.as_slice(), l0.as_slice(), l2.as_slice()];
    let a3 = [p1.as_slice(), p3.as_slice(), p0.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "willr_donch");
    let b: [&[&[f64]]; 2] = [&a0, &a3];
    lanes_match_scalar(&req, &b, "willr_donch L2");
}

#[test]
fn lanes_ichimoku() {
    let req = Request::new().add(Ichimoku::recipe(&[9.0, 26.0]), None);
    let (h0, h2) = (hi(0), hi(2));
    let (l1, l3) = (lo(1), lo(3));
    let p1 = price(1);
    let p2 = price(2);
    let a0 = [h0.as_slice(), l3.as_slice()];
    let a1 = [p2.as_slice(), p1.as_slice()];
    let a2 = [l1.as_slice(), h2.as_slice()];
    let a3 = [l3.as_slice(), h0.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "ichimoku");
}

// ── dispatch honesty ──────────────────────────────────────────────────────

/// A shape no registered pattern covers must fall through the lane dispatch
/// too — never mis-serve.
#[test]
fn lanes_unregistered_shape_falls_back() {
    let req = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Ppo::recipe(&[13.0, 27.0]), None);
    let p0 = price(0);
    let p1 = price(1);
    let a0 = [p0.as_slice()];
    let a1 = [p1.as_slice()];
    let a: [&[&[f64]]; 2] = [&a0, &a1];
    assert!(run_assets_registered(&req, &a).is_none());
}

/// Lanes must agree with the HAND ORACLE directly (not just with the scalar
/// engine) on at least the showcase shape — if both engines shared a drift
/// the hand pin proves the FAMILY, not just the axis. NATR's hand needs
/// high>close bars for the TR leg, so use real h/l/c, not copies.
#[test]
fn lanes_atr_natr_match_hand() {
    let req = Request::new()
        .add(Atr::recipe(&[14.0]), None)
        .add(Natr::recipe(&[14.0]), None);
    // VALID OHLC: the hand seeds bar-0 TR as `high[0] − low[0]` (no abs —
    // it assumes the data contract), so mixed-phase triples are legal for
    // engine-vs-engine parity but NOT for a hand pin. One phase per asset.
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let a0 = [h0.as_slice(), l0.as_slice(), p0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice(), p1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice(), p2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice(), p3.as_slice()];
    let assets3: [[&[f64]; 3]; 4] = [a0, a1, a2, a3];
    let a: [&[&[f64]]; 4] = [&assets3[0], &assets3[1], &assets3[2], &assets3[3]];
    let out = run_assets_registered(&req, &a).expect("lane hit");
    for (k, asset) in assets3.iter().enumerate() {
        let (ar, _) = Atr::indicator(asset, &[14.0], None).unwrap();
        let (nr, _) = Natr::indicator(asset, &[14.0], None).unwrap();
        assert_eq!(out.per_asset[k].rows[0].as_slice(), ar[0], "atr lane {k}");
        assert_eq!(
            out.per_asset[k].rows[2].as_slice(),
            nr[0].as_slice(),
            "natr lane {k}"
        );
    }
}

/// Three-way probe (interpreter / scalar kernel / lane kernel) on the
/// natr+supertrend shape — localizes any divergence to one engine.
#[test]
fn lanes_natr_st_three_way() {
    let req = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let h = hi(1);
    let l = lo(3);
    let c = price(0);
    let asset = [h.as_slice(), l.as_slice(), c.as_slice()];
    let one: [&[&[f64]]; 1] = [&asset];
    let (_a, sig, vals, _o) = erase_request(&req).unwrap();
    let interp = tulip_rs::cse::kernel::run::<SIG_natr_supertrend_shared_p>(&vals, &asset);
    let sc = try_run(&sig, &vals, &asset).expect("scalar hit");
    let lane = run_assets_registered(&req, &one).expect("lane hit");
    for row in 0..interp.len() {
        let ip: &[f64] = interp[row].as_slice();
        let ln: &[f64] = lane.per_asset[0].rows[row].as_slice();
        assert_eq!(ln, ip, "row {row} lane vs interpreter");
        let scv: &[f64] = sc[row].as_slice();
        assert_eq!(scv, ip, "row {row} scalar vs interpreter");
    }
}
