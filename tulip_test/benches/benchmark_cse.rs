//! CSE benchmark — the JIT planner engine vs the hand-rolled indicators.
//!
//! One group per workload, benches registered hand-first then JIT so every
//! pair is adjacent in the report (`group/hand_*` vs `group/jit_*`). The
//! engine is `tulip_rs::cse::jit` (Cranelift): the whole `Request` — merged
//! CSE'd arena + every sink — compiled into ONE fused machine loop. The old
//! interpreted executors (fuse/columnar/simd/hybrid) measured 2.9–7× behind
//! hand and were removed; hand code is the only baseline that matters.
//!
//! Criterion-only (no DB logging).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::cse::{JitPlan, Planable, Request};
use tulip_rs::indicator_types::{Indicator, IndicatorByOptions, TIndicatorState};
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::ppo::Ppo;

const APO: [f64; 2] = [12.0, 26.0];
const MACD: [f64; 3] = [12.0, 26.0, 9.0];
const PPO: [f64; 2] = [12.0, 26.0];
const ASSETS: usize = 8;
const BARS: usize = 5000;

/// Deterministic per-asset synthetic series.
fn asset(i: usize) -> Vec<f64> {
    (0..BARS)
        .map(|b| ((b as f64 + (i as f64) * 3.17) * 0.7).sin() * 10.0 + 100.0)
        .collect()
}

/// The classic merged query (apo + macd + ppo, primary-only emission —
/// `add_opt(_, None)` for parity with the hand `None` flags).
fn classic_request() -> Request {
    Request::new()
        .add_opt(Apo::recipe(&APO), None)
        .add_opt(Macd::recipe(&MACD), None)
        .add_opt(Ppo::recipe(&PPO), None)
}

/// Classic + `ema12`/`ema26` as standalone requests: the arena does NOT
/// grow (same CSE keys — `ema_lines_piggyback_shared_nodes` pins
/// `nodes.len() == 3`), so the two extra lines are two gated stores.
/// The hand path pays two full extra `Ema::indicator` scans.
fn plus_emas_request() -> Request {
    classic_request()
        .add_opt(Ema::recipe(&[12.0]), None)
        .add_opt(Ema::recipe(&[26.0]), None)
}

/// Dense dashboard: 11 distinct-period close EMAs (two Apo + two Ppo + two
/// Macd; `ema9` shared by `Apo(6,9)`+`Ppo(9,25)` → CSE dedups it) plus two
/// chained signal EMAs — the engine's one merged loop vs the hand
/// `indicator_by_options::<2>` lane packs.
fn dense_request() -> Request {
    Request::new()
        .add_opt(Apo::recipe(&[5.0, 8.0]), None)
        .add_opt(Apo::recipe(&[6.0, 9.0]), None)
        .add_opt(Ppo::recipe(&[9.0, 25.0]), None)
        .add_opt(Ppo::recipe(&[10.0, 26.0]), None)
        .add_opt(Macd::recipe(&[11.0, 23.0, 17.0]), None)
        .add_opt(Macd::recipe(&[12.0, 24.0, 9.0]), None)
}

/// Plan-cache miss cost: compile the classic query from scratch per iter.
/// This is the price the cache amortizes — one-time per (shape) process.
fn bench_prepare(c: &mut Criterion) {
    let mut g = c.benchmark_group("prepare");
    g.sample_size(20);
    g.bench_function("jit_compile", |b| {
        b.iter(|| {
            let plan = classic_request().build();
            black_box((plan.arena().nodes.len(), plan.info()))
        })
    });
    g.finish();
}

/// The classic query: one 5000-bar asset, five output lines.
fn bench_classic(c: &mut Criterion) {
    let series = asset(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan: JitPlan = classic_request().build();
    let mut g = c.benchmark_group("classic");
    g.sample_size(20);
    g.bench_function("hand_3_indicators", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &APO, None).unwrap();
            let m = Macd::indicator(&inputs, &MACD, None).unwrap();
            let p = Ppo::indicator(&inputs, &PPO, None).unwrap();
            black_box(a.0[0].len() + m.0[0].len() + p.0[0].len())
        })
    });
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box(qs.next_bar());
            black_box(res.outputs[0][0].len())
        })
    });
    g.bench_function("jit_query_export_states", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            let states = plan.get_states(&qs);
            black_box((res.outputs.len(), states.len()))
        })
    });
    g.finish();
}

/// The dedup showcase: the classic query PLUS standalone `ema12`/`ema26`
/// requests. The JIT pays two gated stores; hand pays two full scans.
fn bench_dedup(c: &mut Criterion) {
    let series = asset(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = plus_emas_request().build();
    let mut g = c.benchmark_group("dedup");
    g.sample_size(20);
    g.bench_function("hand_5_indicators", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &APO, None).unwrap();
            let m = Macd::indicator(&inputs, &MACD, None).unwrap();
            let p = Ppo::indicator(&inputs, &PPO, None).unwrap();
            let e1 = Ema::indicator(&inputs, &[12.0], None).unwrap();
            let e2 = Ema::indicator(&inputs, &[26.0], None).unwrap();
            black_box(a.0[0].len() + m.0[0].len() + p.0[0].len() + e1.0[0].len() + e2.0[0].len())
        })
    });
    g.bench_function("jit_plus_2_emas", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box(qs.next_bar());
            black_box(res.outputs[3][0].len() + res.outputs[4][0].len())
        })
    });
    g.finish();
}

/// The dense dashboard (13 lines from 6 requests). The hand competitor is
/// the status-quo SIMD *option axis*: each indicator packs its own 2 option
/// sets into `Simd<2>` via `indicator_by_options::<2>` — lane-packed
/// recurrences, hand-fused tails, NO cross-indicator dedup. The JIT's edge
/// is CSE + one merged pass; the hand's edge is vectorized chains (until
/// the emitter gets lane packing — Handover Step E1).
fn bench_dense(c: &mut Criterion) {
    let series = asset(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let apo_opts: [&[f64; 2]; 2] = [&[5.0, 8.0], &[6.0, 9.0]];
    let ppo_opts: [&[f64; 2]; 2] = [&[9.0, 25.0], &[10.0, 26.0]];
    let macd_opts: [&[f64; 3]; 2] = [&[11.0, 23.0, 17.0], &[12.0, 24.0, 9.0]];
    let plan = dense_request().build();
    let mut g = c.benchmark_group("dense");
    g.sample_size(20);
    g.bench_function("hand_by_options_2", |b| {
        b.iter(|| {
            let a = Apo::indicator_by_options::<2>(&inputs, &apo_opts, None).unwrap();
            let p = Ppo::indicator_by_options::<2>(&inputs, &ppo_opts, None).unwrap();
            let m = Macd::indicator_by_options::<2>(&inputs, &macd_opts, None).unwrap();
            black_box(a.0[0][0].len() + p.0[0][0].len() + m.0[0][0].len())
        })
    });
    g.bench_function("jit_dashboard", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box(qs.next_bar());
            black_box(res.outputs[0][0].len())
        })
    });
    g.finish();
}

/// Eight assets through the classic query. Hand competitor: the SIMD
/// *asset axis* (`indicator_by_assets::<8>`, lanes = symbols, no
/// cross-indicator dedup).
fn bench_assets(c: &mut Criterion) {
    let series: Vec<Vec<f64>> = (0..ASSETS).map(asset).collect();
    let plan = classic_request().build();
    let inputs: [&[&[f64]; 1]; ASSETS] = [
        &[series[0].as_slice()],
        &[series[1].as_slice()],
        &[series[2].as_slice()],
        &[series[3].as_slice()],
        &[series[4].as_slice()],
        &[series[5].as_slice()],
        &[series[6].as_slice()],
        &[series[7].as_slice()],
    ];
    let mut g = c.benchmark_group("assets");
    g.sample_size(20);
    g.bench_function(&format!("hand_by_assets_{ASSETS}"), |b| {
        b.iter(|| {
            let a = Apo::indicator_by_assets::<ASSETS>(&inputs, &APO, None).unwrap();
            let m = Macd::indicator_by_assets::<ASSETS>(&inputs, &MACD, None).unwrap();
            let p = Ppo::indicator_by_assets::<ASSETS>(&inputs, &PPO, None).unwrap();
            black_box((a.0[0][0].len(), m.0[0][0].len(), p.0[0][0].len()))
        })
    });
    g.bench_function(&format!("jit_{ASSETS}_assets"), |b| {
        b.iter(|| {
            let mut total = 0usize;
            for s in &series {
                let inp: [&[f64]; 1] = [s.as_slice()];
                let (res, _qs) = plan.run(&inp);
                total += res.outputs[0][0].len();
            }
            black_box(total)
        })
    });
    g.finish();
}

/// Tick hot-path: bulk history once, then resume 1 bar/day. The JIT's
/// shared-authority `resume` vs three separate `batch_indicator` calls
/// (the hand streaming floor).
fn bench_tick(c: &mut Criterion) {
    let series = asset(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = classic_request().build();
    let (_r, qs) = plan.run(&inputs);

    let apo = Apo::indicator(&inputs, &APO, None).unwrap().1;
    let macd = Macd::indicator(&inputs, &MACD, None).unwrap().1;
    let ppo = Ppo::indicator(&inputs, &PPO, None).unwrap().1;

    let new_bar = [100.0_f64];
    let nb: [&[f64]; 1] = [new_bar.as_slice()];

    let mut g = c.benchmark_group("tick");
    g.sample_size(20);
    g.bench_function("hand_3_batch", |b| {
        b.iter_batched(
            || (apo.clone(), macd.clone(), ppo.clone()),
            |(mut a, mut m, mut p)| {
                let ra = a.batch_indicator(&nb, None).unwrap();
                let rm = m.batch_indicator(&nb, None).unwrap();
                let rp = p.batch_indicator(&nb, None).unwrap();
                black_box(ra[0].len() + rm[0].len() + rp[0].len())
            },
            criterion::BatchSize::PerIteration,
        )
    });
    g.bench_function("jit_resume", |b| {
        b.iter_batched(
            || qs.clone(),
            |state| {
                let (_res, qs2) = plan.resume(state, &nb);
                black_box(qs2.next_bar())
            },
            criterion::BatchSize::PerIteration,
        )
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_prepare,
    bench_classic,
    bench_dedup,
    bench_dense,
    bench_assets,
    bench_tick
);
criterion_main!(benches);
