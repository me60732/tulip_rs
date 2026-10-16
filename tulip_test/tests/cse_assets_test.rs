//! Asset-lane kernel parity (M2): `<name>_assets::<L>` must reproduce,
//! byte-for-byte, the per-asset scalar `run_registered` result for every
//! registered pattern. The lane axis may change only SPEED, never values —
//! these tests are the correctness gate for the whole asset emitter in
//! `tulip_rs_macros::cse::codegen::run_assets_fn`.

use tulip_rs::cse::*;
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::ad::Ad;
use tulip_rs::indicators::adosc::Adosc;
use tulip_rs::indicators::adx::Adx;
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::aroon::Aroon;
use tulip_rs::indicators::aroonosc::AroonOsc;
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::bbands::BBands;
use tulip_rs::indicators::di::Di;
use tulip_rs::indicators::dm::Dm;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::dx::Dx;
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
use tulip_rs::indicators::ultosc::Ultosc;
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

/// A positive, finite volume column per asset slot — `ad`/`adosc` are the first
/// volume-consuming patterns, so the lane harness needs a fourth stream.
fn vol(k: usize) -> Vec<f64> {
    series(N, |i| {
        1_000.0 + (i as f64 * 0.07 + 0.4 * k as f64).sin() * 400.0 + 400.0 + k as f64
    })
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

// ── THE DIRECTIONAL HUBS on the lane axis ────────────────────────────────
// Nothing in this suite referenced `Dm`/`Di`/`Dx`/`Adx` before, so the lane
// kernels `kernel!` generates for every non-packed pattern had NEVER RUN for
// either hub. The registry comment claimed the mega-hub's assets kernel
// "exists" — a kernel that has never executed is not a verified fact, so these
// two gates are the claim's evidence. The scalar-vs-hand parity gates prove the
// numbers; these prove the lane path reproduces them per asset, which is the
// lane kernel's whole contract.

#[test]
fn lanes_dm_di_dx_adx() {
    let req = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None);
    // Four DISTINCT assets — a lane kernel that reads the wrong asset's carry is
    // invisible if every asset is the same series. Slot 3 is the flat torture
    // case (high == low == close), which zeroes `tr` and drives DI's denominator
    // through the same path on both axes.
    let (h0, h1, h2) = (hi(0), hi(1), hi(2));
    let (l0, l1, l2) = (lo(0), lo(1), lo(2));
    let (c0, c1, c2, c3) = (price(0), price(1), price(2), price(3));
    let a0 = [h0.as_slice(), l0.as_slice(), c0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice(), c1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice(), c2.as_slice()];
    let a3 = [c3.as_slice(), c3.as_slice(), c3.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "dm_di_dx_adx");
}

/// THE FULL DASHBOARD, seven indicators off one plan and four assets. This is
/// the lane path for the widest sharing shape in the registry — the one arena
/// where ONE mean `Wilders(tr)` chain serves FIVE consumers (di, adx, atr, natr,
/// supertrend) and ONE `+DM`/`−DM` graph serves four.
#[test]
fn lanes_dm_di_dx_adx_atr_natr_supertrend() {
    let req = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None)
        .add(Atr::recipe(&[14.0]), None)
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let (h0, h1, h2) = (hi(0), hi(1), hi(2));
    let (l0, l1, l2) = (lo(0), lo(1), lo(2));
    let (c0, c1, c2, c3) = (price(0), price(1), price(2), price(3));
    let a0 = [h0.as_slice(), l0.as_slice(), c0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice(), c1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice(), c2.as_slice()];
    let a3 = [c3.as_slice(), c3.as_slice(), c3.as_slice()];
    let a: [&[&[f64]]; 4] = [&a0, &a1, &a2, &a3];
    lanes_match_scalar(&req, &a, "hub7");
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

/// Ring-backed lanes (step C, 2026-10-13): each asset gets its own
/// `Buffer<Cold>` instance driven by the SAME arena group coordinates —
/// lanes must byte-equal the scalar AO series AND the hand oracle on real
/// hi/lo pairs (lo < hi per asset: the hand AO divides by lo-based
/// denominators, the data contract applies).
#[test]
fn lanes_ao_match_scalar_and_hand() {
    use tulip_rs::indicators::ao::Ao;
    let req = Request::new().add(Ao::recipe(&[]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let a0 = [h0.as_slice(), l0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice()];
    let assets2: [[&[f64]; 2]; 2] = [a0, a1];
    let la2: [&[&[f64]]; 2] = [&assets2[0], &assets2[1]];
    lanes_match_scalar(&req, &la2, "ao L2");
    let assets4: [[&[f64]; 2]; 4] = [a0, a1, a2, a3];
    let la4: [&[&[f64]]; 4] = [&assets4[0], &assets4[1], &assets4[2], &assets4[3]];
    lanes_match_scalar(&req, &la4, "ao L4");
    let out = run_assets_registered(&req, &la4).expect("ao lane hit");
    for (k, asset) in assets4.iter().enumerate() {
        let (ao, _) = Ao::indicator(asset, &[], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            ao[0].as_slice(),
            "ao hand pin asset {k}"
        );
    }
}

/// HMA lanes: the bespoke seven-accumulator machine + its length-√n ring,
/// per asset. Scalar parity + hand pin at one period (the registry's
/// `hma_bit_exact` sweeps the periods for the scalar side).
#[test]
fn lanes_hma_match_scalar_and_hand() {
    use tulip_rs::indicators::hma::Hma;
    let req = Request::new().add(Hma::recipe(&[9.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let assets1: [[&[f64]; 1]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets1[0], &assets1[1], &assets1[2], &assets1[3]];
    lanes_match_scalar(&req, &la, "hma L4");
    let la2: [&[&[f64]]; 2] = [&assets1[0], &assets1[3]];
    lanes_match_scalar(&req, &la2, "hma L2");
    let out = run_assets_registered(&req, &la).expect("hma lane hit");
    for (k, asset) in assets1.iter().enumerate() {
        let (hma, _) = Hma::indicator(asset, &[9.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            hma[0].as_slice(),
            "hma hand pin asset {k}"
        );
    }
}

/// Fisher's lanes run the hand's SIMD `ln_unchecked` (≤ 20 ULP from the
/// libm call the SCALAR arm keeps — §6.41), so fish rows compare 1e-12
/// RELATIVE vs both scalar engine and hand; everything else on the lane
/// path (extremum mirror-window walk, clamp selects, IIR, delayed-copy
/// signal) is bit-equivalent, and the fused pattern's AO row stays exactly
/// bit-equal. `assert_rel_eq` is the ONLY place a lane comparison is not
/// `assert_eq` — the why lives in this doc, forever.
fn assert_rel_eq(got: &[f64], want: &[f64], tag: &str) {
    assert_eq!(
        got.len(),
        want.len(),
        "{tag}: len {} vs {}",
        got.len(),
        want.len()
    );
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        let scale = g.abs().max(w.abs()).max(1.0);
        assert!(
            (g - w).abs() <= 1e-12 * scale,
            "{tag} idx {i}: {g:e} vs {w:e}"
        );
    }
}

#[test]
fn lanes_fisher_match_scalar_and_hand() {
    use tulip_rs::indicators::fisher::Fisher;
    let req = Request::new().add(Fisher::recipe(&[9.0]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let a0 = [h0.as_slice(), l0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice()];
    let assets: [[&[f64]; 2]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    let out = run_assets_registered(&req, &la).expect("fisher lane hit");
    for (a, asset) in assets.iter().enumerate() {
        let want = run_registered(&req, asset).expect("fisher scalar");
        for r in 0..2 {
            assert_rel_eq(
                out.per_asset[a].rows[r].as_slice(),
                want.rows[r].as_slice(),
                &format!("fisher lane {a} row {r}"),
            );
        }
        let (fish, _) = Fisher::indicator(asset, &[9.0], None).unwrap();
        for r in 0..2 {
            assert_rel_eq(
                out.per_asset[a].rows[r].as_slice(),
                fish[r].as_slice(),
                &format!("fisher hand pin lane {a} row {r}"),
            );
        }
    }
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    let out2 = run_assets_registered(&req, &la2).expect("fisher L2 hit");
    let pairs: [&[&[f64]; 2]; 2] = [&assets[0], &assets[3]];
    for (k, asset) in pairs.into_iter().enumerate() {
        let want = run_registered(&req, asset).expect("fisher scalar");
        for r in 0..2 {
            assert_rel_eq(
                out2.per_asset[k].rows[r].as_slice(),
                want.rows[r].as_slice(),
                &format!("fisher L2 lane {k} row {r}"),
            );
        }
    }
}

#[test]
fn lanes_ao_fisher_shared_med() {
    use tulip_rs::indicators::ao::Ao;
    use tulip_rs::indicators::fisher::Fisher;
    let req = Request::new()
        .add(Ao::recipe(&[]), None)
        .add(Fisher::recipe(&[9.0]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let a0 = [h0.as_slice(), l0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice()];
    let assets: [[&[f64]; 2]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    let out = run_assets_registered(&req, &la).expect("fused lane hit");
    for (a, asset) in assets.iter().enumerate() {
        let want = run_registered(&req, asset).expect("fused scalar");
        // Sink layout (per recipe, primaries then optionals, request order):
        // [0]=ao, [1..4]=ao optionals (UNASKED ⇒ zero-length), [4]=fish,
        // [5]=signal — the layout the merge canary pins for erase. The AO
        // row is pure sum machinery: bit-equal across axes is mandatory.
        assert_eq!(
            out.per_asset[a].rows[0].as_slice(),
            want.rows[0].as_slice(),
            "fused ao lane {a} (bit-exact)"
        );
        assert_eq!(out.per_asset[a].rows.len(), 6, "sink count");
        for r in 1..4 {
            assert!(out.per_asset[a].rows[r].is_empty(), "unasked optional {r}");
        }
        for r in [4usize, 5] {
            assert_rel_eq(
                out.per_asset[a].rows[r].as_slice(),
                want.rows[r].as_slice(),
                &format!("fused lane {a} vs scalar fish row {r}"),
            );
        }
        let (ao_h, _) = Ao::indicator(asset, &[], None).unwrap();
        let (fi_h, _) = Fisher::indicator(asset, &[9.0], None).unwrap();
        assert_eq!(
            out.per_asset[a].rows[0].as_slice(),
            ao_h[0].as_slice(),
            "fused ao hand lane {a}"
        );
        for (r, h) in [(4usize, 0usize), (5, 1)] {
            assert_rel_eq(
                out.per_asset[a].rows[r].as_slice(),
                fi_h[h].as_slice(),
                &format!("fused fisher hand lane {a} row {r}"),
            );
        }
    }
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

/// Hand-oracle pin for the SuperTrend pattern on DISTINCT valid OHLC —
/// the registry's hand parity for natr+st feeds `[s,s,s]` copies, which are
/// blind to band/TR wiring; here the lane kernel is pinned to the hand on
/// real (high, low, close) triples, one phase per asset.
#[test]
fn lanes_natr_st_match_hand() {
    let req = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let (h, l, c) = (hi(1), lo(1), price(1));
    let (h2, l2, c2) = (hi(3), lo(3), price(3));
    let a0 = [h.as_slice(), l.as_slice(), c.as_slice()];
    let a1 = [h2.as_slice(), l2.as_slice(), c2.as_slice()];
    let assets: [[&[f64]; 3]; 2] = [a0, a1];
    let la: [&[&[f64]]; 2] = [&assets[0], &assets[1]];
    let out = run_assets_registered(&req, &la).expect("lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (nr, _) = Natr::indicator(asset, &[14.0], None).unwrap();
        let (st, _) = SuperTrend::indicator(asset, &[14.0, 3.0], None).unwrap();
        // sinks: [natr, …, …, supertrend] primaries (see registry None-flag test)
        assert_eq!(out.per_asset[k].rows[0].as_slice(), nr[0], "natr lane {k}");
        assert_eq!(out.per_asset[k].rows[3].as_slice(), st[0], "st lane {k}");
    }
}

/// Stochastic on the lane axis: the batched engine's two ring GROUPS
/// (slow %K over the division column with skip:2, %D over the slow line)
/// warm per-asset and replay ONCE at the seam, then step with one
/// batched feed + eviction/at read per group per bar. No `ln` anywhere —
/// byte-for-byte identity vs the scalar engine AND the hand oracle
/// (real OHLC per asset: close strictly inside the hi/lo bracket).
#[test]
fn lanes_stoch_match_scalar_and_hand() {
    use tulip_rs::indicators::stoch::Stoch;
    let req = Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let a0 = [h0.as_slice(), l0.as_slice(), p0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice(), p1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice(), p2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice(), p3.as_slice()];
    let assets: [[&[f64]; 3]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "stoch L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "stoch L2");
    let out = run_assets_registered(&req, &la).expect("stoch lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (hand, _) = Stoch::indicator(asset, &[14.0, 3.0, 3.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            hand[0].as_slice(),
            "stoch %K hand lane {k}"
        );
        assert_eq!(
            out.per_asset[k].rows[1].as_slice(),
            hand[1].as_slice(),
            "stoch %D hand lane {k}"
        );
    }
}

/// The flagship triple on lanes: stoch + willr + donchian share the ONE
/// interned (14, 13) roll pair per column — two price-stream extrema walks
/// + two ring groups feed eight output rows per asset. Byte-equal across
/// axes; %K/%D additionally pinned to the hand.
#[test]
fn lanes_stoch_willr_donchian_shared() {
    use tulip_rs::indicators::stoch::Stoch;
    let req = Request::new()
        .add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)
        .add(Willr::recipe(&[14.0]), None)
        .add(DonchianChannel::recipe(&[14.0]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let a0 = [h0.as_slice(), l0.as_slice(), p0.as_slice()];
    let a1 = [h1.as_slice(), l1.as_slice(), p1.as_slice()];
    let a2 = [h2.as_slice(), l2.as_slice(), p2.as_slice()];
    let a3 = [h3.as_slice(), l3.as_slice(), p3.as_slice()];
    let assets: [[&[f64]; 3]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "stoch_wd L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "stoch_wd L2");
    let out = run_assets_registered(&req, &la).expect("triple lane hit");
    // stoch [k, d], willr primary + two UNASKED optional mirrors,
    // donchian [lower, middle, upper].
    assert_eq!(out.per_asset[0].rows.len(), 8, "sink count: 2 + 1+2 + 3");
    for (k, asset) in assets.iter().enumerate() {
        let (hand, _) = Stoch::indicator(asset, &[14.0, 3.0, 3.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            hand[0].as_slice(),
            "triple %K hand lane {k}"
        );
        assert_eq!(
            out.per_asset[k].rows[1].as_slice(),
            hand[1].as_slice(),
            "triple %D hand lane {k}"
        );
    }
}

/// RSI on the lane axis: dual mean-Wilders chains over the Max-gated
/// movement arms plus the `prev_real` state mirror (identity copy, DCE'd —
/// the lane hit proves `Kind::Mirror` lowers cleanly on the batched path
/// too). No `ln` anywhere ⇒ BYTE equality vs scalar AND hand on four price
/// phases (sine/sine/plateau/ramp).
#[test]
fn lanes_rsi_match_scalar_and_hand() {
    use tulip_rs::indicators::rsi::Rsi;
    let req = Request::new().add(Rsi::recipe(&[14.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let assets: [[&[f64]; 1]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "rsi L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "rsi L2");
    let out = run_assets_registered(&req, &la).expect("rsi lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (rsi, _) = Rsi::indicator(asset, &[14.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            rsi[0].as_slice(),
            "rsi hand pin lane {k}"
        );
    }
}

/// The flagship pair on the lane axis: Rsi + StochRsi co-requested — the
/// grafted movement chain interns ONCE, the optional rsi row aliases the
/// primary, and the pair's extrema rolls walk one derived ring per asset
/// (the batched lane engine on the second column). No `ln` anywhere ⇒ byte
/// equality vs scalar AND both hands, L4 + L2 buckets.
#[test]
fn lanes_rsi_stochrsi_shared_match_scalar_and_hand() {
    use tulip_rs::indicators::rsi::Rsi;
    use tulip_rs::indicators::stochrsi::StochRsi;
    let req = Request::new()
        .add(Rsi::recipe(&[14.0]), None)
        .add(StochRsi::recipe(&[14.0]), Some(&[true]));
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let assets: [[&[f64]; 1]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "rsi_stochrsi L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "rsi_stochrsi L2");
    let out = run_assets_registered(&req, &la).expect("pair lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (rsi, _) = Rsi::indicator(asset, &[14.0], None).unwrap();
        let (sr, _) = StochRsi::indicator(asset, &[14.0], Some(&[true])).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            rsi[0].as_slice(),
            "pair rsi hand lane {k}"
        );
        assert_eq!(
            out.per_asset[k].rows[1].as_slice(),
            sr[0].as_slice(),
            "pair stochrsi hand lane {k}"
        );
        assert_eq!(
            out.per_asset[k].rows[2].as_slice(),
            sr[1].as_slice(),
            "pair rsi-mirror hand lane {k}"
        );
    }
}

/// CMO on the lane axis: twin ring groups (gain/loss sums over the derived
/// Max-gated arms) warmed per asset and replayed once at the seam, then ONE
/// batched feed + eviction read per group per bar; the two state Mirrors ride
/// as DCE'd identities. Raw sums (mults 1.0), no `ln` ⇒ BYTE equality vs
/// scalar AND hand, L4 + L2.
#[test]
fn lanes_cmo_match_scalar_and_hand() {
    use tulip_rs::indicators::cmo::Cmo;
    let req = Request::new().add(Cmo::recipe(&[14.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let assets: [[&[f64]; 1]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "cmo L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "cmo L2");
    let out = run_assets_registered(&req, &la).expect("cmo lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (cmo, _) = Cmo::indicator(asset, &[14.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            cmo[0].as_slice(),
            "cmo hand pin lane {k}"
        );
    }
}

/// ULTOSC on lanes — the shape the B-axis container was built for. `bp` and
/// `tr` are two independent pure-sum groups of THREE members each, so the
/// planner names ONE `MultiBuffer<2>` (one push, ONE cursor advance) for all
/// six folds; the long member's age equals capacity, so its pre-push read IS
/// the eviction. The hand answers the same problem with `MultiBuffer<2>`
/// itself (`ultosc.rs:9`), and its bar-0 skip comes from the arena for free:
/// `Lag{close,1}` gives `bp`/`tr` lead 1, and a derived roll feeds at
/// `feeds[k] = lead(src)`.
///
/// Valid OHLC, one phase per asset — mixed-phase triples are legal for
/// engine-vs-engine but not for a hand pin (see `lanes_atr_natr_match_hand`).
/// `ultosc` is the only PRIMARY row here (`tr`/`bp` are optional sinks), and its
/// lead is `long_period` per the hand's `min_data`.
#[test]
fn lanes_ultosc_match_scalar_and_hand() {
    const OPTS: [f64; 3] = [7.0, 14.0, 28.0];
    let req = Request::new().add(Ultosc::recipe(&OPTS), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let assets3: [[&[f64]; 3]; 4] = [
        [h0.as_slice(), l0.as_slice(), p0.as_slice()],
        [h1.as_slice(), l1.as_slice(), p1.as_slice()],
        [h2.as_slice(), l2.as_slice(), p2.as_slice()],
        [h3.as_slice(), l3.as_slice(), p3.as_slice()],
    ];
    let a: [&[&[f64]]; 4] = [&assets3[0], &assets3[1], &assets3[2], &assets3[3]];
    lanes_match_scalar(&req, &a, "ultosc L4");
    let b: [&[&[f64]]; 2] = [&assets3[1], &assets3[2]];
    lanes_match_scalar(&req, &b, "ultosc L2");
    let out = run_assets_registered(&req, &a).expect("ultosc lane hit");
    for (k, asset) in assets3.iter().enumerate() {
        let (u, _) = Ultosc::indicator(asset, &OPTS, None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            u[0].as_slice(),
            "ultosc hand pin lane {k}"
        );
    }
}

/// The movement-sharing pair on lanes: Rsi + Cmo co-requested intern the
/// whole `up_down` stage (`Lag{1}`, both Subs, both Max gates, the shared
/// `Mirror{real}`) — two Wilders chains and two ring groups off ONE movement
/// compute, two sinks. Byte equality vs scalar AND both hands, L4 + L2.
#[test]
fn lanes_rsi_cmo_shared_match_scalar_and_hand() {
    use tulip_rs::indicators::cmo::Cmo;
    use tulip_rs::indicators::rsi::Rsi;
    let req = Request::new()
        .add(Rsi::recipe(&[14.0]), None)
        .add(Cmo::recipe(&[14.0]), None);
    let p = prices();
    let a0 = [p[0].as_slice()];
    let a1 = [p[1].as_slice()];
    let a2 = [p[2].as_slice()];
    let a3 = [p[3].as_slice()];
    let assets: [[&[f64]; 1]; 4] = [a0, a1, a2, a3];
    let la: [&[&[f64]]; 4] = [&assets[0], &assets[1], &assets[2], &assets[3]];
    lanes_match_scalar(&req, &la, "rsi_cmo L4");
    let la2: [&[&[f64]]; 2] = [&assets[0], &assets[3]];
    lanes_match_scalar(&req, &la2, "rsi_cmo L2");
    let out = run_assets_registered(&req, &la).expect("rsi+cmo lane hit");
    for (k, asset) in assets.iter().enumerate() {
        let (rsi, _) = Rsi::indicator(asset, &[14.0], None).unwrap();
        let (cmo, _) = Cmo::indicator(asset, &[14.0], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            rsi[0].as_slice(),
            "rsi_cmo rsi hand lane {k}"
        );
        assert_eq!(
            out.per_asset[k].rows[1].as_slice(),
            cmo[0].as_slice(),
            "rsi_cmo cmo hand lane {k}"
        );
    }
}

/// AD on lanes: the first pattern whose recurrence multiplies by a COLUMN that
/// is not a price, so this is the gate for the lane `Accum` arm (one fused
/// `mul_add` per lane, carry seeded per asset) and the lane `WhereLe` arm.
///
/// The second call deliberately feeds `high == low`: `range` is then `0 <= eps`
/// on EVERY bar, so the guard must take the pinned zero and the accumulator
/// must stay `+0.0` end-to-end. That is what makes the guard's lane arm tested
/// rather than merely compiled — take the value arm by mistake and `numer/0.0`
/// is `NaN`, which poisons the carry for every later bar and every lane.
#[test]
fn lanes_ad_match_scalar_and_hand() {
    let req = Request::new().add(Ad::recipe(&[]), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let (v0, v1, v2, v3) = (vol(0), vol(1), vol(2), vol(3));
    let assets4: [[&[f64]; 4]; 4] = [
        [h0.as_slice(), l0.as_slice(), p0.as_slice(), v0.as_slice()],
        [h1.as_slice(), l1.as_slice(), p1.as_slice(), v1.as_slice()],
        [h2.as_slice(), l2.as_slice(), p2.as_slice(), v2.as_slice()],
        [h3.as_slice(), l3.as_slice(), p3.as_slice(), v3.as_slice()],
    ];
    let a: [&[&[f64]]; 4] = [&assets4[0], &assets4[1], &assets4[2], &assets4[3]];
    lanes_match_scalar(&req, &a, "ad L4");
    let b: [&[&[f64]]; 2] = [&assets4[1], &assets4[2]];
    lanes_match_scalar(&req, &b, "ad L2");

    let out = run_assets_registered(&req, &a).expect("ad lane hit");
    for (k, asset) in assets4.iter().enumerate() {
        let (ad, _) = Ad::indicator(asset, &[], None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            ad[0].as_slice(),
            "ad hand pin lane {k}"
        );
    }

    // The guard torture set: high == low on every bar of every lane.
    let flat: [[&[f64]; 4]; 4] = [
        [p0.as_slice(), p0.as_slice(), p0.as_slice(), v0.as_slice()],
        [p1.as_slice(), p1.as_slice(), p1.as_slice(), v1.as_slice()],
        [p2.as_slice(), p2.as_slice(), p2.as_slice(), v2.as_slice()],
        [p3.as_slice(), p3.as_slice(), p3.as_slice(), v3.as_slice()],
    ];
    let fa: [&[&[f64]]; 4] = [&flat[0], &flat[1], &flat[2], &flat[3]];
    lanes_match_scalar(&req, &fa, "ad L4 flat");
    let fout = run_assets_registered(&req, &fa).expect("ad flat lane hit");
    for (k, asset) in flat.iter().enumerate() {
        let (ad, _) = Ad::indicator(asset, &[], None).unwrap();
        assert_eq!(
            fout.per_asset[k].rows[0].as_slice(),
            ad[0].as_slice(),
            "ad flat hand pin lane {k}"
        );
        // And the guard really does hold the carry at its zero init.
        assert!(
            fout.per_asset[k].rows[0]
                .iter()
                .all(|v| v.to_bits() == 0u64),
            "ad flat lane {k}: every bar must be +0.0"
        );
    }
}

/// ADOSC on lanes — the shape the old JIT lost badly on: the RECURRENCE reads a
/// derived, volume-driven source (both `Ema`s chain onto the AD accumulator)
/// while the co-requested `Apo`/`Ppo` read close. Lanes must reproduce the
/// scalar plan byte-for-byte on the primary row and both optional EMA rows.
#[test]
fn lanes_adosc_match_scalar_and_hand() {
    const OPTS: [f64; 2] = [3.0, 10.0];
    let req = Request::new().add(Adosc::recipe(&OPTS), None);
    let (h0, h1, h2, h3) = (hi(0), hi(1), hi(2), hi(3));
    let (l0, l1, l2, l3) = (lo(0), lo(1), lo(2), lo(3));
    let (p0, p1, p2, p3) = (price(0), price(1), price(2), price(3));
    let (v0, v1, v2, v3) = (vol(0), vol(1), vol(2), vol(3));
    let assets4: [[&[f64]; 4]; 4] = [
        [h0.as_slice(), l0.as_slice(), p0.as_slice(), v0.as_slice()],
        [h1.as_slice(), l1.as_slice(), p1.as_slice(), v1.as_slice()],
        [h2.as_slice(), l2.as_slice(), p2.as_slice(), v2.as_slice()],
        [h3.as_slice(), l3.as_slice(), p3.as_slice(), v3.as_slice()],
    ];
    let a: [&[&[f64]]; 4] = [&assets4[0], &assets4[1], &assets4[2], &assets4[3]];
    lanes_match_scalar(&req, &a, "adosc L4");
    let b: [&[&[f64]]; 2] = [&assets4[1], &assets4[2]];
    lanes_match_scalar(&req, &b, "adosc L2");

    let out = run_assets_registered(&req, &a).expect("adosc lane hit");
    for (k, asset) in assets4.iter().enumerate() {
        let (os, _) = Adosc::indicator(asset, &OPTS, None).unwrap();
        assert_eq!(
            out.per_asset[k].rows[0].as_slice(),
            os[0].as_slice(),
            "adosc hand pin lane {k}"
        );
    }

    // Guard torture: high == low ⇒ the AD line is pinned at zero, so both EMA
    // chains step on zeros and the oscillator must be exactly zero throughout.
    let flat: [[&[f64]; 4]; 4] = [
        [p0.as_slice(), p0.as_slice(), p0.as_slice(), v0.as_slice()],
        [p1.as_slice(), p1.as_slice(), p1.as_slice(), v1.as_slice()],
        [p2.as_slice(), p2.as_slice(), p2.as_slice(), v2.as_slice()],
        [p3.as_slice(), p3.as_slice(), p3.as_slice(), v3.as_slice()],
    ];
    let fa: [&[&[f64]]; 4] = [&flat[0], &flat[1], &flat[2], &flat[3]];
    lanes_match_scalar(&req, &fa, "adosc L4 flat");
    let fout = run_assets_registered(&req, &fa).expect("adosc flat lane hit");
    for k in 0..4 {
        assert!(
            fout.per_asset[k].rows[0]
                .iter()
                .all(|v| v.abs().to_bits() == 0u64),
            "adosc flat lane {k}: zero ad ⇒ zero oscillator"
        );
    }
}

/// THE DIRECTIONAL HUB on the lane axis: `dm`+`di`+`dx`+`adx` over one DM chain.
/// Two claims, checked separately.
///
/// (1) `lanes_match_scalar` pins every lane byte-identical to the scalar kernel
/// on the same asset. Hand parity then follows BY TRANSITIVITY from
/// `dm_di_dx_adx_shared_p_bit_exact` (scalar rows vs all four hands) — so this
/// gate deliberately does NOT re-derive sink indices: the hub has nine sinks
/// whose order is a property of the merge, and hard-coding positions here would
/// fail for wiring reasons while the numbers stayed right.
///
/// (2) The lane path is where the composite/mean pair is actually stressed:
/// `di`'s raw-sum ATR and `adx`'s mean ATR are Wilders over ONE interned `tr`
/// node, and the lane group reads `vals.invms[k]` PER LANE (unity vs 1/period).
/// A lane result equal to the scalar result is therefore evidence that the
/// mean/composite choice travels as a magnitude, not as a spelling.
#[test]
fn lanes_dm_di_dx_adx_hub() {
    use tulip_rs::indicators::adx::Adx;
    use tulip_rs::indicators::di::Di;
    use tulip_rs::indicators::dm::Dm;
    use tulip_rs::indicators::dx::Dx;

    let req = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None);

    let (h, l, c) = (hi(1), lo(1), price(1));
    let (h2, l2, c2) = (hi(3), lo(3), price(3));
    let assets: [[&[f64]; 3]; 2] = [
        [h.as_slice(), l.as_slice(), c.as_slice()],
        [h2.as_slice(), l2.as_slice(), c2.as_slice()],
    ];
    let la: [&[&[f64]]; 2] = [&assets[0], &assets[1]];
    lanes_match_scalar(&req, &la, "dm+di+dx+adx hub L2");

    // Degenerate prices: high == low == close makes every DM delta zero, so the
    // two COMPOSITE accumulators fold to 0 and `dx` becomes 0/0. The hub's `Max`
    // is IEEE maxNum (`f64::max`), which is what turns that NaN into the hand's
    // 0.0 — and a lane/scalar divergence here would mean the two paths pick the
    // NaN rule differently, which no finite-price-only comparison can reveal.
    let flat: [[&[f64]; 3]; 2] = [
        [c.as_slice(), c.as_slice(), c.as_slice()],
        [c2.as_slice(), c2.as_slice(), c2.as_slice()],
    ];
    let fa: [&[&[f64]]; 2] = [&flat[0], &flat[1]];
    lanes_match_scalar(&req, &fa, "dm+di+dx+adx hub L2 flat");
}
