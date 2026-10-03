//! JIT ablation ladder: same 5000-bar series, plans that add one structural
//! feature at a time, so a regression between rows names the culprit.
//!
//!   1 ema12        — 1 chain, 1 gated store                (the floor)
//!   2 ema12/26     — 2 chains, 2 gated stores              (chain + store step)
//!   3 apo          — 2 chains, +tail value, 1 store        (tail arithmetic)
//!   4 apo+ppo      — 2 chains, 2 tails, 2 stores
//!   5 macd         — 2 chains, +CHAINED sig (select/seed), 3 stores
//!   6 classic      — everything, 5 stores                  (the shipped query)
//!   8 hand_*       — `::indicator()` calls                  (the competitors)
//!
//! NOTE: a branchless-store variant (always store, select the address) was
//! tried and measured 18→24µs on classic — the gated store chain stays.
//! Absolute times drift with core selection on this hybrid CPU; run
//! `cargo bench -p tulip_test --bench benchmark_cse` for trustworthy numbers.
//!
//! Run: `cargo run -p tulip_test --release --example jit_ablation`

use std::time::Instant;
use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::ppo::Ppo;

const BARS: usize = 5000;
const ROUNDS: usize = 2000;

fn series() -> Vec<f64> {
    (0..BARS)
        .map(|b| ((b as f64) * 0.7).sin() * 10.0 + 100.0)
        .collect()
}

fn time(label: &str, mut f: impl FnMut()) {
    // warm u
    for _ in 0..50 {
        f();
    }
    let t = Instant::now();
    for _ in 0..ROUNDS {
        f();
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / ROUNDS as f64;
    println!("{label:<22} {us:>8.2} µs/run");
}

fn main() {
    let close = series();
    let s: [&[f64]; 1] = [close.as_slice()];

    let ema = Request::new().add(Ema::recipe(&[12.0]), None).build();
    let ema2 = Request::new()
        .add(Ema::recipe(&[12.0]), None)
        .add(Ema::recipe(&[26.0]), None)
        .build();
    let apo = Request::new().add(Apo::recipe(&[12.0, 26.0]), None).build();
    let apo_ppo = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Ppo::recipe(&[12.0, 26.0]), None)
        .build();
    let macd = Request::new()
        .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
        .build();
    let classic = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
        .add(Ppo::recipe(&[12.0, 26.0]), None)
        .build();

    time("1 jit ema12", || {
        let (r, q) = ema.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    time("2 jit ema12+26", || {
        let (r, q) = ema2.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    time("3 jit apo", || {
        let (r, q) = apo.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    time("4 jit apo+ppo", || {
        let (r, q) = apo_ppo.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    time("5 jit macd", || {
        let (r, q) = macd.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    time("6 jit classic", || {
        let (r, q) = classic.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });

    // Hand competitor, same allocation shape (fresh vecs per call).
    time("8 hand apo", || {
        std::hint::black_box(Apo::indicator(&s, &[12.0, 26.0], None).unwrap().0[0].len());
    });
    time("8 hand macd", || {
        std::hint::black_box(Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0[0].len());
    });
    time("8 hand ppo", || {
        std::hint::black_box(Ppo::indicator(&s, &[12.0, 26.0], None).unwrap().0[0].len());
    });
    time("8 hand all 3", || {
        let a = Apo::indicator(&s, &[12.0, 26.0], None).unwrap();
        let m = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap();
        let p = Ppo::indicator(&s, &[12.0, 26.0], None).unwrap();
        std::hint::black_box(a.0[0].len() + m.0[0].len() + p.0[0].len());
    });

    // E2 asset lanes: same classic query, 8 symbols in one fused plan
    // (same series fed to every lane — shape is what times).
    let lanes8 = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
        .add(Ppo::recipe(&[12.0, 26.0]), None)
        .build_assets(8);
    println!(
        "assets8 lanes: {:?} ({} vector subs)",
        lanes8.lanes(),
        lanes8.vec_subs()
    );
    time("7 jit assets8 lanes", || {
        let ss: Vec<&[f64]> = (0..8).map(|_| close.as_slice()).collect();
        let (r, q) = lanes8.run_assets(&ss);
        std::hint::black_box(r[0].outputs[0][0].len() + q.words.len());
    });

    std::fs::write("/tmp/assets8.clif", lanes8.clif()).ok();

    // Dump the classic CLIF for inspection.
    std::fs::write("/tmp/classic.clif", classic.clif()).ok();
    println!("classic CLIF written to /tmp/classic.clif");

    // ...and the dense dashboard's, to audit the extract/insert traffic.
    let dense = Request::new()
        .add(Apo::recipe(&[5.0, 8.0]), None)
        .add(Apo::recipe(&[6.0, 9.0]), None)
        .add(Ppo::recipe(&[9.0, 25.0]), None)
        .add(Ppo::recipe(&[10.0, 26.0]), None)
        .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
        .build();
    println!("dense lanes: {:?}", dense.lanes());
    time("9 jit dense", || {
        let (r, q) = dense.run(&s);
        std::hint::black_box(r.outputs[0][0].len() + q.words.len());
    });
    std::fs::write("/tmp/dense.clif", dense.clif()).ok();
    println!("dense CLIF written to /tmp/dense.clif");
}
