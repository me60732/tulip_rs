//! Differential parity harness (integration test — its own binary, so the
//! ~N plans it compiles can't crowd the lib-test process's shared cranelift
//! executable-memory arena). For every wired recipe × data regime (incl. the
//! degenerate ones that hid the Max/stddev boundary bugs) × pseudo-random
//! period, asserts jit cold == hand cold (all primary rows) AND jit streamed
//! == jit cold (the cold==resumed invariant).
//!
//! The JIT plan's stream arity is derived from the built plan (`n_streams`);
//! the hand's input arity is declared per case (its `INPUTS`), which can
//! exceed what the plan reads (e.g. Dx validates 3 price inputs but the JIT
//! only consumes high/low). Stream layout: 1→[close]; 2→[high,low]; 3→[h,l,c].
//! Deterministic PRNG -> no `proptest` dependency.

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::adx::Adx;
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::aroon::Aroon;
use tulip_rs::indicators::aroonosc::AroonOsc;
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::bbands::BBands;
use tulip_rs::indicators::cmo::Cmo;
use tulip_rs::indicators::di::Di;
use tulip_rs::indicators::dm::Dm;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::dx::Dx;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::hma::Hma;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::medprice::Medprice;
use tulip_rs::indicators::min::Min;
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::ppo::Ppo;
use tulip_rs::indicators::rsi::Rsi;
use tulip_rs::indicators::sma::Sma;
use tulip_rs::indicators::stddev::StdDev;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_rs::indicators::wilders::Wilders;
use tulip_rs::indicators::willr::Willr;
use tulip_rs::indicators::wma::Wma;

const N: usize = 600;

fn walk(seed: u64) -> Vec<f64> {
    let mut s = seed | 1;
    let mut x = 100.0f64;
    (0..N)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            x += ((s % 200) as f64 - 100.0) * 0.05;
            x.max(1.0)
        })
        .collect()
}

fn regimes() -> Vec<(&'static str, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let closes: Vec<(&str, Vec<f64>)> = vec![
        (
            "smooth",
            (0..N)
                .map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
                .collect(),
        ),
        ("saw", (0..N).map(|i| 80.0 + (i % 37) as f64).collect()),
        (
            "spike",
            (0..N)
                .map(|i| if i % 50 == 0 { 200.0 } else { 100.0 })
                .collect(),
        ),
        ("flat", vec![100.0; N]),
        ("big-lowvar", (0..N).map(|i| 1e8 + (i % 3) as f64).collect()),
        ("walk", walk(0x9E37_79B9_7F4A_7C15)),
        ("walk2", walk(0x1234_5678_9ABC_DEF1)),
    ];
    closes
        .into_iter()
        .map(|(l, c)| {
            let h: Vec<f64> = c
                .iter()
                .enumerate()
                .map(|(i, v)| v + (i % 5) as f64 + 1.0)
                .collect();
            let lo: Vec<f64> = c.iter().map(|v| (v - 1.0 - (v % 3.0)).max(0.5)).collect();
            (l, h, lo, c)
        })
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

fn pick<'a>(ns: usize, h: &'a [f64], l: &'a [f64], c: &'a [f64]) -> Vec<&'a [f64]> {
    match ns {
        1 => vec![c],
        2 => vec![h, l],
        _ => vec![h, l, c],
    }
}

/// `hand_ns` = the hand indicator's input arity (its `INPUTS`); the JIT plan
/// arity comes from `n_streams()` (may be smaller). Both are laid out by the
/// same `pick` convention so each reads the prices it expects.
fn check(
    name: &str,
    regime: &str,
    p: usize,
    h: &[f64],
    l: &[f64],
    c: &[f64],
    hand_ns: usize,
    mk: &dyn Fn(usize) -> Request,
    hand: &dyn Fn(&[&[f64]]) -> Vec<Vec<f64>>,
) {
    let plan = mk(p).build();
    let inp = pick(plan.n_streams(), h, l, c);
    let (res, _) = plan.run(&inp);
    let rr = res.request(0);
    let want = hand(&pick(hand_ns, h, l, c));
    let n = rr.output_count();
    assert!(
        n <= want.len(),
        "{name} {regime}/p{p}: jit {n} > hand {}",
        want.len()
    );
    for i in 0..n {
        cmp_exact(
            rr.output(i),
            &want[i],
            &format!("{name} {regime}/p{p} row{i}"),
        );
    }
    let sp = N / 2;
    let cut: Vec<&[f64]> = inp.iter().map(|s| &s[..sp]).collect();
    let tail: Vec<&[f64]> = inp.iter().map(|s| &s[sp..]).collect();
    let plan2 = mk(p).build();
    let (r1, st1) = plan2.run(&cut);
    let r2 = plan2.resume(st1, &tail).0;
    for i in 0..n {
        let mut cat = r1.request(0).output(i).to_vec();
        cat.extend_from_slice(r2.request(0).output(i));
        cmp_exact(
            &cat,
            rr.output(i),
            &format!("{name} {regime} stream/p{p} row{i}"),
        );
    }
}

#[test]
fn differential_parity() {
    let mut rng = 0x243F_6A88_85A3_08D3u64;
    let mut period = move |span: u64| {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        5 + (rng % span) as usize
    };

    for (regime, h, l, c) in regimes() {
        for _ in 0..3 {
            let p = period(20);
            macro_rules! chk {
                ($nm:expr, $hn:expr, $mk:expr, $hd:expr) => {
                    check(
                        $nm,
                        regime,
                        p,
                        &h,
                        &l,
                        &c,
                        $hn,
                        &move |q| $mk(q),
                        &move |s| $hd(s, p),
                    )
                };
            }
            // name, hand arity, plan builder, hand rows(s) with period via cap
            chk!(
                "Ema",
                1,
                |q: usize| Request::new().add(Ema::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Ema::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Sma",
                1,
                |q: usize| Request::new().add(Sma::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Sma::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Wma",
                1,
                |q: usize| Request::new().add(Wma::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Wma::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Hma",
                1,
                |q: usize| Request::new().add(Hma::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Hma::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Wilders",
                1,
                |q: usize| Request::new().add(Wilders::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Wilders::indicator(&[s[0]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "StdDev",
                1,
                |q: usize| Request::new().add(StdDev::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| StdDev::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "BBands",
                1,
                |q: usize| Request::new().add(BBands::recipe(&[q as f64, 2.0]), None),
                |s: &[&[f64]], pp: usize| BBands::indicator(&[s[0]], &[pp as f64, 2.0], None)
                    .unwrap()
                    .0
            );
            // E9 trail-rescan rolls: the regimes are the point — `flat`
            // (every bar ties), `saw` (the extremum repeats every 37 bars)
            // and `big-lowvar` (1e8 + i%3 ties under a 1-ulp window) all put
            // the newest-occurrence rule and the rescan schedule under
            // pressure, which a value-only comparison would never see.
            chk!(
                "Max",
                1,
                |q: usize| Request::new().add(Max::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Max::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Min",
                1,
                |q: usize| Request::new().add(Min::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Min::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            // E9 M1: the trail consumers. Both rows (down/up) are compared, and
            // `saw`/`flat`/`big-lowvar` are the regimes where `trail` (the thing
            // aroon actually prints) is decided by the tie rule.
            chk!(
                "Aroon",
                2,
                |q: usize| Request::new().add(Aroon::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Aroon::indicator(&[s[0], s[1]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "AroonOsc",
                2,
                |q: usize| Request::new().add(AroonOsc::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| AroonOsc::indicator(&[s[0], s[1]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            // E9 M2: the (p, p−1)-width range consumers. `flat` is the
            // degenerate-range guard (max == min → the division must be
            // discarded for 0.0 by the WhereGe select).
            chk!(
                "Willr",
                3,
                |q: usize| Request::new().add(Willr::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Willr::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "Rsi",
                1,
                |q: usize| Request::new().add(Rsi::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Rsi::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "Cmo",
                1,
                |q: usize| Request::new().add(Cmo::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Cmo::indicator(&[s[0]], &[pp as f64], None).unwrap().0
            );
            chk!(
                "DonchianChannel",
                2,
                |q: usize| Request::new().add(DonchianChannel::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| DonchianChannel::indicator(
                    &[s[0], s[1]],
                    &[pp as f64],
                    None
                )
                .unwrap()
                .0
            );
            chk!(
                "Apo",
                1,
                |q: usize| Request::new().add(Apo::recipe(&[q as f64, (q + 10) as f64]), None),
                |s: &[&[f64]], pp: usize| Apo::indicator(
                    &[s[0]],
                    &[pp as f64, (pp + 10) as f64],
                    None
                )
                .unwrap()
                .0
            );
            chk!(
                "Ppo",
                1,
                |q: usize| Request::new().add(Ppo::recipe(&[q as f64, (q + 10) as f64]), None),
                |s: &[&[f64]], pp: usize| Ppo::indicator(
                    &[s[0]],
                    &[pp as f64, (pp + 10) as f64],
                    None
                )
                .unwrap()
                .0
            );
            chk!(
                "Macd",
                1,
                |q: usize| Request::new()
                    .add(Macd::recipe(&[q as f64, (q + 12) as f64, 9.0]), None),
                |s: &[&[f64]], pp: usize| Macd::indicator(
                    &[s[0]],
                    &[pp as f64, (pp + 12) as f64, 9.0],
                    None
                )
                .unwrap()
                .0
            );
            chk!(
                "Medprice",
                2,
                |_: usize| Request::new().add(Medprice::recipe(&[]), None),
                |s: &[&[f64]], _pp: usize| Medprice::indicator(&[s[0], s[1]], &[], None).unwrap().0
            );
            chk!(
                "Dm",
                2,
                |q: usize| Request::new().add(Dm::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Dm::indicator(&[s[0], s[1]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "Atr",
                3,
                |q: usize| Request::new().add(Atr::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Atr::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "Natr",
                3,
                |q: usize| Request::new().add(Natr::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Natr::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "Di",
                3,
                |q: usize| Request::new().add(Di::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Di::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            // Dx: hand takes 3 inputs (validates close) but the plan reads 2.
            chk!(
                "Dx",
                3,
                |q: usize| Request::new().add(Dx::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Dx::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "Adx",
                3,
                |q: usize| Request::new().add(Adx::recipe(&[q as f64]), None),
                |s: &[&[f64]], pp: usize| Adx::indicator(&[s[0], s[1], s[2]], &[pp as f64], None)
                    .unwrap()
                    .0
            );
            chk!(
                "SuperTrend",
                3,
                |q: usize| Request::new().add(SuperTrend::recipe(&[q as f64, 3.0]), None),
                |s: &[&[f64]], pp: usize| SuperTrend::indicator(
                    &[s[0], s[1], s[2]],
                    &[pp as f64, 3.0],
                    None
                )
                .unwrap()
                .0
            );
        }
    }
}
