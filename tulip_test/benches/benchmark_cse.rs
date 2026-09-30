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
use tulip_rs::cse::{JitPlan, Planable, Recipe, Request};
use tulip_rs::indicator_types::{Indicator, IndicatorByOptions, TIndicatorState};
use tulip_rs::indicators::adx::Adx;
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::bbands::BBands;
use tulip_rs::indicators::di::Di;
use tulip_rs::indicators::dm::Dm;
use tulip_rs::indicators::dx::Dx;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::hma::Hma;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::medprice::Medprice;
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::ppo::Ppo;
use tulip_rs::indicators::sma::Sma;
use tulip_rs::indicators::stddev::StdDev;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_rs::indicators::tr::Tr;
use tulip_rs::indicators::wilders::Wilders;
use tulip_rs::indicators::wma::Wma;

use tulip_test::database::{get_all_stock_data, init_database_data, EodData};

const APO: [f64; 2] = [12.0, 26.0];
const MACD: [f64; 3] = [12.0, 26.0, 9.0];
const PPO: [f64; 2] = [12.0, 26.0];
/// Real EOD symbols available (BHP/CBA/AAPL/MSFT in the synthetic fallback;
/// whatever the DB returns once login is wired). The asset-axis bench uses one
/// per lane, so it caps at this.
const ASSETS: usize = 4;

/// Real EOD data — the SAME source the per-indicator benches use
/// (`init_database_data` is an idempotent `OnceLock`, so per-bench calls are
/// free after the first). With `should_log_to_db=false` it is the canonical
/// synthetic 15-bar→1500 EodData; with login wired it is real Postgres EOD,
/// and every bench below switches to real data with no further edit.
fn stocks() -> &'static Vec<(String, Vec<EodData>)> {
    init_database_data();
    get_all_stock_data().expect("stock data initialised")
}
fn col(sym: usize, f: impl Fn(&EodData) -> f64) -> Vec<f64> {
    stocks()[sym].1.iter().map(|d| f(d)).collect()
}
/// One real price stream (single-input indicators run on close).
fn close(sym: usize) -> Vec<f64> {
    col(sym, |d| d.close)
}
/// Real (high, low, close) triple for the multi-input benches.
fn hlc(sym: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    (
        col(sym, |d| d.high),
        col(sym, |d| d.low),
        col(sym, |d| d.close),
    )
}

/// The classic merged query (apo + macd + ppo, primary-only emission —
/// `add_opt(_, None)` for parity with the hand `None` flags).
/// classic query with EVERY declared optional mirrored on (E2c's shape).
fn classic_request_all_on() -> Request {
    Request::new()
        .add_all(Apo::recipe(&APO))
        .add_all(Macd::recipe(&MACD))
        .add_all(Ppo::recipe(&PPO))
}

fn classic_request() -> Request {
    Request::new()
        .add(Apo::recipe(&APO), None)
        .add(Macd::recipe(&MACD), None)
        .add(Ppo::recipe(&PPO), None)
}

/// Classic + `ema12`/`ema26` as standalone requests: the arena does NOT
/// grow (same CSE keys — `ema_lines_piggyback_shared_nodes` pins
/// `nodes.len() == 3`), so the two extra lines are two gated stores.
/// The hand path pays two full extra `Ema::indicator` scans.
fn plus_emas_request() -> Request {
    classic_request()
        .add(Ema::recipe(&[12.0]), None)
        .add(Ema::recipe(&[26.0]), None)
}

/// Dense dashboard: 11 distinct-period close EMAs (two Apo + two Ppo + two
/// Macd; `ema9` shared by `Apo(6,9)`+`Ppo(9,25)` → CSE dedups it) plus two
/// chained signal EMAs — the engine's one merged loop vs the hand
/// `indicator_by_options::<2>` lane packs.
fn dense_request() -> Request {
    Request::new()
        .add(Apo::recipe(&[5.0, 8.0]), None)
        .add(Apo::recipe(&[6.0, 9.0]), None)
        .add(Ppo::recipe(&[9.0, 25.0]), None)
        .add(Ppo::recipe(&[10.0, 26.0]), None)
        .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
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
    // E4: the same through the plan cache — warm lookup (the first
    // iteration pays the compile, amortized into criterion's warmup).
    g.bench_function("jit_cached_build", |b| {
        b.iter(|| {
            let plan = classic_request().build_cached();
            black_box((plan.arena().nodes.len(), plan.info(), plan.fingerprint()))
        })
    });
    g.finish();
}

/// The classic query: one 5000-bar asset, five output lines.
fn bench_classic(c: &mut Criterion) {
    let series = close(0);
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
    // E2c showcase: ALL optional outputs on. 11 request slots, but only 6
    // distinct output nodes — the JIT writes each row once and hands
    // mirror slots aliased views; the hand trio writes 11 arrays (its own
    // duplicates). Hand parity of values AND lengths is pinned in cse tests.
    let on_flags = Some(&[true, true][..]);
    let plan_on = classic_request_all_on().build();
    g.bench_function("hand_3_indicators_all_on", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &APO, on_flags).unwrap();
            let m = Macd::indicator(&inputs, &MACD, on_flags).unwrap();
            let p = Ppo::indicator(&inputs, &PPO, on_flags).unwrap();
            black_box(a.0.len() + m.0.len() + p.0.len())
        })
    });
    g.bench_function("jit_query_all_on", |b| {
        b.iter(|| {
            let (res, qs) = plan_on.run(&inputs);
            black_box((
                qs.next_bar(),
                res.outputs[0][0].len(),
                res.optional[2][1].len(),
            ))
        })
    });
    g.finish();
}

/// The dedup showcase: the classic query PLUS standalone `ema12`/`ema26`
/// requests. The JIT pays two gated stores; hand pays two full scans.
fn bench_dedup(c: &mut Criterion) {
    let series = close(0);
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
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let apo_opts: [&[f64; 2]; 2] = [&[5.0, 8.0], &[6.0, 9.0]];
    let ppo_opts: [&[f64; 2]; 2] = [&[9.0, 25.0], &[10.0, 26.0]];
    let macd_opts: [&[f64; 3]; 2] = [&[11.0, 23.0, 17.0], &[12.0, 24.0, 9.0]];
    let plan = dense_request().build();
    let mut g = c.benchmark_group("dense");
    g.sample_size(20);
    g.bench_function("hand_by_options_2(Dashboard)", |b| {
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

/// Four assets (real symbols) through the classic query. Hand competitor:
/// the SIMD *asset axis* (`indicator_by_assets::<4>`, lanes = symbols, no
/// cross-indicator dedup).
fn bench_assets(c: &mut Criterion) {
    // one real symbol per lane; `run_assets`/`indicator_by_assets` require all
    // streams equal length, so trim to the shortest real symbol.
    let mut series: Vec<Vec<f64>> = (0..ASSETS).map(close).collect();
    let len = series.iter().map(|s| s.len()).min().unwrap();
    for s in &mut series {
        s.truncate(len);
    }
    let plan = classic_request().build();
    let inputs: [&[&[f64]; 1]; ASSETS] = [
        &[series[0].as_slice()],
        &[series[1].as_slice()],
        &[series[2].as_slice()],
        &[series[3].as_slice()],
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
    // E2: ONE asset-lane plan for all 4 symbols. Beyond the xmm pressure
    // (4 group vecs + 4 sigs), E2b's carried write-pointers put many GP
    // values through the int file — spills every bar, and this
    // shape got WORSE under the walk. KEPT as the measurement of that cliff.
    let lanes_plan = classic_request().build_assets(ASSETS);
    g.bench_function(&format!("jit_{ASSETS}_assets_lanes"), |b| {
        b.iter(|| {
            let ss: Vec<&[f64]> = series.iter().map(|v| v.as_slice()).collect();
            let (res, _qs) = lanes_plan.run_assets(&ss);
            black_box((res.len(), res[0].outputs[0][0].len()))
        })
    });
    // E2 sweet spot at 2 lanes: asset-PAIR plans (A = 2). Under E2b this
    // shape came WITHIN 3.5% of sequential (56.9 vs 55.0µs): the walk
    // feeds 10 carried pointers to the int file and frees xmm pressure, so
    // cross-asset pairing finally pays — still not a win at classic's 2
    // chains (the pairing budget argument stands), but the gap moved from
    // 12% to noise. One fused loop steps two symbols; two pairs cover 4.
    let pair_plan = classic_request().build_assets(2);
    g.bench_function(&format!("jit_{ASSETS}_assets_paired"), |b| {
        b.iter(|| {
            let mut total = 0usize;
            for pr in 0..ASSETS / 2 {
                let ss: [&[f64]; 2] = [series[2 * pr].as_slice(), series[2 * pr + 1].as_slice()];
                let (res, _qs) = pair_plan.run_assets(&ss);
                total += res[0].outputs[0][0].len() + res[1].outputs[0][0].len();
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
    let series = close(0);
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
    // Streaming surface (§3.2): advance one bar writing slot values into a
    // caller buffer — reuses the SAME compiled loop as `jit_resume` but with a
    // stack scratch instead of a heap arena + `Results` tree. This is the
    // zero-allocation tick the 117 ns resume floor was measured against.
    let nslots = plan.n_slots();
    g.bench_function("jit_tick", |b| {
        b.iter_batched(
            || qs.clone(),
            |mut state| {
                let mut out = [0.0_f64; 32];
                plan.tick(&mut state, &nb, &mut out[..nslots]);
                black_box(out[0] + out[nslots - 1])
            },
            criterion::BatchSize::PerIteration,
        )
    });
    // E4: the per-tick worst case done RIGHT — rebuild the request every
    // tick (a stateless caller), served by the plan cache instead of a
    // 411µs compile.
    g.bench_function("jit_cached_resume", |b| {
        b.iter_batched(
            || qs.clone(),
            |state| {
                let p = classic_request().build_cached();
                let (_res, qs2) = p.resume(state, &nb);
                black_box(qs2.next_bar())
            },
            criterion::BatchSize::PerIteration,
        )
    });
    g.finish();
}

/// E3 rolling trio (sma + stddev + bbands @20): the hand calls three
/// windows over the same real series — three passes, three sums; the JIT
/// fused loop runs ONE ring + ONE sum (shared via CSE across the three
/// requests) and mirrors the middle line out of it.
fn bench_rolling(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = Request::new()
        .add(Sma::recipe(&[20.0]), None)
        .add(StdDev::recipe(&[20.0]), None)
        .add(BBands::recipe(&[20.0, 2.0]), None)
        .add(Wma::recipe(&[20.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling");
    g.sample_size(20);
    g.bench_function("hand_3_rolling_indicators", |b| {
        b.iter(|| {
            let sm = Sma::indicator(&inputs, &[20.0], None).unwrap();
            let sd = StdDev::indicator(&inputs, &[20.0], None).unwrap();
            let bb = BBands::indicator(&inputs, &[20.0, 2.0], None).unwrap();
            let w = Wma::indicator(&inputs, &[20.0], None).unwrap();
            black_box(sm.0[0].len() + sd.0[0].len() + bb.0[0].len() + w.0[0].len())
        })
    });
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    g.finish();
}

/// E3 stage 3: HMA — the engine's ONLY per-bar stored-CALCULATION ring
/// (the psqrt=4 diff buffer for period 20). Hand pays one full pass per
/// call; the fused loop rides the same anchor-stitched price window as
/// the other rolling kinds and pays the ring push/pop for the diff line
/// alone. The `jit_with_sma` row proves the deliberate NON-share: an
/// Hma(20) + Sma(20) query duplicates the period-20 sum roll (~4 flops/
/// bar) rather than couple the bespoke op to the group machinery.
fn bench_rolling_hma(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = Request::new().add(Hma::recipe(&[20.0]), None).build();
    let plan_sma = Request::new()
        .add(Hma::recipe(&[20.0]), None)
        .add(Sma::recipe(&[20.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_hma");
    g.sample_size(20);
    g.bench_function("hand_hma", |b| {
        b.iter(|| {
            let h = Hma::indicator(&inputs, &[20.0], None).unwrap();
            //let s = Sma::indicator(&inputs, &[20.0], None).unwrap();
            black_box(h.0[0].len())
        })
    });
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    g.bench_function("jit_with_sma", |b| {
        b.iter(|| {
            let (res, qs) = plan_sma.run(&inputs);
            black_box((
                res.outputs[0][0].len() + res.outputs[1][0].len(),
                qs.next_bar(),
            ))
        })
    });
    g.finish();
}

/// Multiple HMA OPTIONS — the option axis on the bespoke op. Unlike the
/// window-20 trio (which shares one sum+ring), independent HMAs share
/// NOTHING: each owns its two engines + `√P` diff ring. The rivals: the
/// hand's scalar path (one cold pass each) AND the hand's own option-axis
/// SIMD (`indicator_by_options::<N>`), which is the hand's strongest form
/// here — BUT `N` must be a real SIMD lane width (2/4/8), since the lanes
/// pack a `Simd<f64, N>`; `::<3>` is not a valid instantiation (its extra
/// lane is padding, and a `::<3>` timing read ~1µs of nothing). So this
/// row uses **N = 4**. The fused JIT loop is the point of comparison: four
/// self-contained HMA states carried at once test whether the anchor+group
/// machinery's fusion win survives when there is NOTHING to fuse.
fn bench_rolling_hma_multi(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = Request::new()
        .add(Hma::recipe(&[9.0]), None)
        .add(Hma::recipe(&[12.0]), None)
        .add(Hma::recipe(&[20.0]), None)
        .add(Hma::recipe(&[50.0]), None)
        .build();
    let hma_opts: [&[f64; 1]; 4] = [&[9.0], &[12.0], &[20.0], &[50.0]];
    let mut g = c.benchmark_group("rolling_hma_multi");
    g.sample_size(20);
    // hand, scalar axis: one full pass per period over the real series
    g.bench_function("hand_scalar_hma_4", |b| {
        b.iter(|| {
            let a = Hma::indicator(&inputs, &[9.0], None).unwrap();
            let b = Hma::indicator(&inputs, &[20.0], None).unwrap();
            let c = Hma::indicator(&inputs, &[50.0], None).unwrap();
            let d = Hma::indicator(&inputs, &[12.0], None).unwrap();
            black_box(a.0[0].len() + b.0[0].len() + c.0[0].len() + d.0[0].len())
        })
    });
    // hand, option axis: ONE pass, 4 periods lane-packed (f64x4 = a real
    // SIMD width) — the hand's best shape for this workload
    g.bench_function("hand_by_options_4", |b| {
        b.iter(|| {
            let a = Hma::indicator_by_options::<4>(&inputs, &hma_opts, None).unwrap();
            black_box(a.0[0][0].len() + a.0[1][0].len() + a.0[2][0].len() + a.0[3][0].len())
        })
    });
    // JIT: all four HMA kernels fused into one resumable loop
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box((
                res.outputs[0][0].len()
                    + res.outputs[1][0].len()
                    + res.outputs[2][0].len()
                    + res.outputs[3][0].len(),
                qs.next_bar(),
            ))
        })
    });
    g.finish();
}

/// SIZES the "HMA.state1 joins the (src,period) group" dedup prize,
/// honestly. The ONLY clean share is `Hma(P).state1` ↔ a co-resident
/// standalone `Wma(P)`/`Sma(P)`: both warm from `real[0..P]`, so they run
/// the identical `sum += src − src[i−P]` (+ `ws` trick for Wma) recurrence.
/// (NOT two HMAs — a `state2` warms from an OFFSET window, so `Hma(20)`
/// and `Hma(40)` do not share a Wma(20); different fold ⇒ sums never
/// reconverge in f64. That is also why HMA can't be composed from
/// standalone nodes.) The prize unit is "one WMA engine in the fused loop":
/// `wma20_wma30 − wma_dup` (wma_dup is Wma(20)+Wma(20), which CSE already
/// merges to ONE engine — the control that proves a genuinely-shared engine
/// is free). Compare `hma20_wma20` (today: two bodies; ideal: the Wma(20)
/// body disappears into HMA's own state1) against `hma20` alone: the gap is
/// the standalone Wma(20)'s marginal cost, which is ~all removable.
fn bench_hma_group_dedup(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    // WMA engines: the prize unit + the free-dedup control.
    let wma20 = Request::new().add(Wma::recipe(&[20.0]), None).build();
    let wma_dup = Request::new()
        .add(Wma::recipe(&[20.0]), None)
        .add(Wma::recipe(&[20.0]), None)
        .build();
    let wma20_30 = Request::new()
        .add(Wma::recipe(&[20.0]), None)
        .add(Wma::recipe(&[30.0]), None)
        .build();
    // The actual dedup target: HMA(20) shares its state1 Wma(20) engine.
    let hma20 = Request::new().add(Hma::recipe(&[20.0]), None).build();
    let hma20_wma20 = Request::new()
        .add(Hma::recipe(&[20.0]), None)
        .add(Wma::recipe(&[20.0]), None)
        .build();
    let hma20_sma20 = Request::new()
        .add(Hma::recipe(&[20.0]), None)
        .add(Sma::recipe(&[20.0]), None)
        .build();
    let mut g = c.benchmark_group("hma_group_dedup");
    g.sample_size(20);
    g.bench_function("jit_wma20", |b| {
        b.iter(|| {
            let (r, qs) = wma20.run(&inputs);
            black_box((r.outputs[0][0].len(), qs.next_bar()))
        })
    });
    // CONTROL: two Wma(20) requests — CSE merges to one engine, so ≈ wma20.
    g.bench_function("jit_wma_dup", |b| {
        b.iter(|| {
            let (r, qs) = wma_dup.run(&inputs);
            black_box((r.outputs[0][0].len() + r.outputs[1][0].len(), qs.next_bar()))
        })
    });
    // PRIZE UNIT: a second, non-colliding engine costs (this − wma20).
    g.bench_function("jit_wma20_wma30", |b| {
        b.iter(|| {
            let (r, qs) = wma20_30.run(&inputs);
            black_box((r.outputs[0][0].len() + r.outputs[1][0].len(), qs.next_bar()))
        })
    });
    g.bench_function("jit_hma20", |b| {
        b.iter(|| {
            let (r, qs) = hma20.run(&inputs);
            black_box((r.outputs[0][0].len(), qs.next_bar()))
        })
    });
    // HMA(20)+Wma(20): today pays a whole extra Wma body; ideal folds the
    // Wma(20) into HMA's own state1 engine → saves ≈ the prize unit.
    g.bench_function("jit_hma20_wma20", |b| {
        b.iter(|| {
            let (r, qs) = hma20_wma20.run(&inputs);
            black_box((r.outputs[0][0].len() + r.outputs[1][0].len(), qs.next_bar()))
        })
    });
    // HMA(20)+Sma(20): shares only the raw SUM roll (Sma has no ws trick),
    // so a smaller partial prize — included to bound the two cases apart.
    g.bench_function("jit_hma20_sma20", |b| {
        b.iter(|| {
            let (r, qs) = hma20_sma20.run(&inputs);
            black_box((r.outputs[0][0].len() + r.outputs[1][0].len(), qs.next_bar()))
        })
    });
    g.finish();
}

/// E6 leaf: Wilders smoothing (single price source) — the ADX family's
/// shared decayed-average chain. Standalone it faces the same "nothing to
/// fuse, one hand tight loop vs one cranelift loop" ceiling as a lone Ema/
/// Hma; the row is the baseline the atr/adx stack will ride on (each ATR/
/// ADX step reuses this exact recurrence, so a whole ADX query is Wilders
/// chains over TR/DX sources — once the (stream,asset) multi-input axis
/// lets TR/ATR exist, `jit_atr_over_wilders` becomes the fusion case).
fn bench_rolling_wilders(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let single = Request::new().add(Wilders::recipe(&[14.0]), None).build();
    // two independent Wilders chains (different periods) in ONE fused loop
    let dual = Request::new()
        .add(Wilders::recipe(&[14.0]), None)
        .add(Wilders::recipe(&[28.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_wilders");
    g.sample_size(20);
    g.bench_function("hand_wilders", |b| {
        b.iter(|| {
            let w = Wilders::indicator(&inputs, &[14.0], None).unwrap();
            black_box(w.0[0].len())
        })
    });
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = single.run(&inputs);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    // dual: two chains fused — the hand pays two passes, the loop one.
    g.bench_function("hand_wilders_2", |b| {
        b.iter(|| {
            let a = Wilders::indicator(&inputs, &[14.0], None).unwrap();
            let c = Wilders::indicator(&inputs, &[28.0], None).unwrap();
            black_box(a.0[0].len() + c.0[0].len())
        })
    });
    g.bench_function("jit_query_dual", |b| {
        b.iter(|| {
            let (res, qs) = dual.run(&inputs);
            black_box((
                res.outputs[0][0].len() + res.outputs[1][0].len(),
                qs.next_bar(),
            ))
        })
    });
    g.finish();
}

/// E6 chained-Wilders: ATR over (H,L,C). Standalone atr is the expected
/// single-vs-single floor (one hand tight loop vs one cranelift loop — the
/// engine isn't trying to win there). The PRIZE row is `jit_tr_plus_atr`:
/// a `tr`+`atr` query shares the identical `tr` subgraph (CSE dedups the
/// nodes), so ONE fused loop rolls true range once for both — where the hand
/// pays a full `tr` pass AND a full `atr` pass (two independent folds over
/// the whole series). That workload reduction, not loop-local scheduling,
/// is where the JIT earns its keep.
fn bench_rolling_atr(c: &mut Criterion) {
    let (hi, lo, cl) = hlc(0);
    let inputs: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let single = Request::new().add(Atr::recipe(&[14.0]), None).build();
    // tr + atr(14) share the tr subgraph → one fused loop, tr rolled once.
    let shared = Request::new()
        .add(Tr::recipe(&[]), None)
        .add(Atr::recipe(&[14.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_atr");
    g.sample_size(20);
    g.bench_function("hand_atr", |b| {
        b.iter(|| {
            let a = Atr::indicator(&inputs, &[14.0], None).unwrap();
            black_box(a.0[0].len())
        })
    });
    g.bench_function("jit_atr", |b| {
        b.iter(|| {
            let (res, qs) = single.run(&inputs);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    // shared: hand rolls tr once (full pass) AND atr once (its own full tr
    // fold) — two passes; the fused plan rolls tr ONCE feeding both sinks.
    g.bench_function("hand_tr_plus_atr", |b| {
        b.iter(|| {
            let t = Tr::indicator(&inputs, &[], None).unwrap();
            let a = Atr::indicator(&inputs, &[14.0], None).unwrap();
            black_box(t.0[0].len() + a.0[0].len())
        })
    });
    g.bench_function("jit_tr_plus_atr", |b| {
        b.iter(|| {
            let (res, qs) = shared.run(&inputs);
            black_box((
                res.outputs[0][0].len() + res.outputs[1][0].len(),
                qs.next_bar(),
            ))
        })
    });
    g.finish();
}

/// E6 ADX family — the fusion prize, AND its boundary. `dm`+`di`+`dx`+`adx`
/// ALL read the same `+DM`/`−DM` composite-accumulator subgraph (deduped by
/// CSE) plus the shared TR→sum-ATR, so ONE fused loop rolls directional
/// movement once for all four; the hand runs four INDEPENDENT full passes (each
/// re-folds the DM chain) → the JIT wins ~1.5×. The `natr`+`adx` pair is the
/// CONTRAST: they share the stateless `tr` subgraph (both fold it) but NOT the
/// ATR recurrence — natr's is the MEAN/calc `Wilders(tr)`, adx's internal one is
/// the SUM/partial-calc `Wilders(tr)` (different `composite` nodes). So the only
/// dedup is `tr` (~7 ops), a small slice of two otherwise-heavy chains → the
/// fused loop serializes both bodies (register pressure) and the hand wins.
/// Lesson: fusion pays off when the SHARED work is a LARGE fraction of the cost
/// (the DM chain ×4), not a minor stateless prefix between two big chains.
/// (A `natr`+`atr` pair would be meaningless — `atr` is `natr`'s own optional
/// output, so no dashboard emits both as separate indicators.)
fn bench_rolling_adx(c: &mut Criterion) {
    let (hi, lo, cl) = hlc(0);
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let two: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
    // PRIMARY-ONLY everywhere (`add(_, None)`) — the default workload a
    // dashboard requests; optionals only ON when the bench is specifically
    // about the optional switch. Both hand + jit sides emit the SAME rows.
    // trend: dm+di+dx+adx all read the SAME +DM/−DM accumulator subgraph
    // (CSE-deduped) → one fused loop rolls directional movement once, where
    // the hand runs four INDEPENDENT full passes. THIS is the shared-chain win.
    let trend = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None)
        .build();
    // natr + adx: their VALUES share nothing (adx is dm→dx→adx-mean; natr is
    // tr→mean-ATR) — only the raw price streams. The honest "no shared chain"
    // contrast to `trend`.
    let vol = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_adx");
    g.sample_size(20);
    g.bench_function("hand_dm_di_dx_adx", |b| {
        b.iter(|| {
            let a = Dm::indicator(&two, &[14.0], None).unwrap();
            let d = Di::indicator(&three, &[14.0], None).unwrap();
            let x = Dx::indicator(&three, &[14.0], None).unwrap();
            let s = Adx::indicator(&three, &[14.0], None).unwrap();
            black_box((a.0[0].len(), d.0[0].len(), x.0[0].len(), s.0[0].len()))
        })
    });
    g.bench_function("jit_dm_di_dx_adx", |b| {
        b.iter(|| {
            let (res, qs) = trend.run(&three);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    g.bench_function("hand_natr_plus_adx", |b| {
        b.iter(|| {
            let n = Natr::indicator(&three, &[14.0], None).unwrap();
            let a = Adx::indicator(&three, &[14.0], None).unwrap();
            black_box((n.0[0].len(), a.0[0].len()))
        })
    });
    g.bench_function("jit_natr_plus_adx", |b| {
        b.iter(|| {
            let (res, qs) = vol.run(&three);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    g.finish();
}

/// E7 multi-value feedback op + its fusion payoff. Standalone `supertrend`
/// is the expected floor (one hand tight loop vs one cranelift loop). The
/// PRIZE is `natr`+`supertrend`: BOTH read the mean `Wilders(tr)` ATR node and
/// the `tr`/`medprice` subgraphs, so CSE dedups them to ONE chain — a
/// `natr`+`supertrend` query computes true range AND the ATR fold ONCE, where
/// the hand runs two passes each recomputing them. Unlike natr+adx (disjoint
/// ATR recurrences), here the shared work is the FULL atr chain, so the fused
/// loop should win.
fn bench_rolling_st(c: &mut Criterion) {
    let (hi, lo, cl) = hlc(0);
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let st_only = Request::new()
        .add(SuperTrend::recipe(&[14.0, 3.0]), None)
        .build();
    // natr + supertrend: BOTH consume the mean `Wilders(tr)` ATR node → the
    // fused loop dedups the WHOLE atr chain (one level up from `tr`), where
    // the hand runs two passes each recomputing tr + the atr fold. Primary-only
    // both sides (`add(_, None)`).
    let vol_trend = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None)
        //.add(Atr::recipe(&[14.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_st");
    g.sample_size(20);
    g.bench_function("hand_supertrend", |b| {
        b.iter(|| {
            let s = SuperTrend::indicator(&three, &[14.0, 3.0], None).unwrap();
            black_box(s.0[0].len())
        })
    });
    g.bench_function("jit_supertrend", |b| {
        b.iter(|| {
            let (res, qs) = st_only.run(&three);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    g.bench_function("hand_natr_plus_supertrend", |b| {
        b.iter(|| {
            let n = Natr::indicator(&three, &[14.0], None).unwrap();
            let s = SuperTrend::indicator(&three, &[14.0, 3.0], None).unwrap();
            //let a = Atr::indicator(&three, &[14.0], None).unwrap();
            black_box((n, s))
        })
    });
    g.bench_function("jit_natr_plus_supertrend", |b| {
        b.iter(|| {
            let (res, qs) = vol_trend.run(&three);
            black_box((res, qs))
        })
    });
    g.finish();
}

/// E6 field axis proof at scale: MEDPRICE (H/L, 2 streams) alone, and
/// MEDPRICE + a close-EMA fused (mixes a multi-input stateless op with a
/// single-input chain in one loop — the shape every ADX-family query has:
/// H/L/C ops alongside close-only chains). This is what `tr → atr → adx`
/// will look like once emitted; medprice is the simplest same-bar case.
fn bench_multi_input(c: &mut Criterion) {
    let (hi, lo, _) = hlc(0);
    let two: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
    let med = Request::new().add(Medprice::recipe(&[]), None).build();
    let fused = Request::new()
        .add(Medprice::recipe(&[]), None)
        .add(Ema::recipe(&[12.0]), None)
        .build();
    let mut g = c.benchmark_group("multi_input");
    g.sample_size(20);
    g.bench_function("hand_medprice", |b| {
        b.iter(|| {
            let m = Medprice::indicator(&two, &[], None).unwrap();
            black_box(m.0[0].len())
        })
    });
    g.bench_function("jit_medprice", |b| {
        b.iter(|| {
            let (res, qs) = med.run(&two);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });
    // fused: 2-field medprice + a close-field chain in ONE loop. n_fields=2
    // (medprice's high/low) so the caller supplies exactly 2 streams; the
    // EMA reads field 0 — the ADX-query shape (H/L/C ops + chains, one loop).
    g.bench_function("jit_medprice_plus_ema", |b| {
        b.iter(|| {
            let (res, qs) = fused.run(&two);
            black_box((
                res.outputs[0][0].len() + res.outputs[1][0].len(),
                qs.next_bar(),
            ))
        })
    });
    g.finish();
}

/// The MIXED-window rolling dashboard (the question single-window benches
/// can't ask): five distinct windows 5/8/14/20/26 across four kinds.
/// Hand pays one full pass per indicator; the plan pays per GROUP for
/// recurrences and per ROW for stores — 12 output slots, 10 write streams.
fn bench_rolling_mixed(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let plan = Request::new()
        .add(Sma::recipe(&[5.0]), None)
        .add(Wma::recipe(&[8.0]), None)
        .add(StdDev::recipe(&[14.0]), None)
        .add(Sma::recipe(&[20.0]), None)
        .add(StdDev::recipe(&[20.0]), None)
        .add(BBands::recipe(&[20.0, 2.5]), None)
        .add(Wma::recipe(&[26.0]), None)
        .build();
    let mut g = c.benchmark_group("rolling_mixed");
    g.sample_size(20);
    g.bench_function("hand_7_indicators", |b| {
        b.iter(|| {
            //let t = Some(&[true][..]);
            let a = Sma::indicator(&inputs, &[5.0], None).unwrap();
            let b = Wma::indicator(&inputs, &[8.0], None).unwrap();
            let c = StdDev::indicator(&inputs, &[14.0], None).unwrap();
            let d = Sma::indicator(&inputs, &[20.0], None).unwrap();
            let e = StdDev::indicator(&inputs, &[20.0], None).unwrap();
            let f = BBands::indicator(&inputs, &[20.0, 2.5], None).unwrap();
            let h = Wma::indicator(&inputs, &[26.0], None).unwrap();
            black_box(
                a.0.len() + b.0.len() + c.0.len() + d.0.len() + e.0.len() + f.0.len() + h.0.len(),
            )
        })
    });
    // The hand's BEST form on this workload, matching the dense group's
    // convention: the option axis lanes the paired windows (sma 5/20,
    // stddev 14/20, wma 8/26) two-per-pass; bbands' single set rides ::<1>.
    let sma_opts: [&[f64; 1]; 2] = [&[5.0], &[20.0]];
    let sd_opts: [&[f64; 1]; 2] = [&[14.0], &[20.0]];
    let wma_opts: [&[f64; 1]; 2] = [&[8.0], &[26.0]];
    //let bb_opts: [&[f64; 2]; 1] = [&[20.0, 2.5]];
    g.bench_function("hand_by_options_2", |b| {
        b.iter(|| {
            let t = Some(&[true][..]);
            let a = Sma::indicator_by_options::<2>(&inputs, &sma_opts, None).unwrap();
            let c = StdDev::indicator_by_options::<2>(&inputs, &sd_opts, t).unwrap();
            let w = Wma::indicator_by_options::<2>(&inputs, &wma_opts, t).unwrap();
            let f = BBands::indicator(&inputs, &[20.0, 2.5], None).unwrap();
            black_box(a.0[0][0].len() + c.0[0][0].len() + w.0[0][0].len() + f.0[0].len())
        })
    });
    g.bench_function("jit_query", |b| {
        b.iter(|| {
            let (res, qs) = plan.run(&inputs);
            black_box((res.outputs[5][2].len(), qs.next_bar()))
        })
    });
    g.finish();
}

/// The `sharing_groups` measurement (see `Merged::sharing_groups`): a mixed
/// dashboard = an EMA cluster (Apo/Macd/Ppo/Ema on close) + an ATR cluster
/// (Atr/Natr/SuperTrend on H/L/C). The two clusters share **no computed node**,
/// so `sharing_groups` yields 2 groups. We time the SAME set of indicators
/// three ways to isolate the "one monolithic fused loop vs one loop per sharing
/// cluster" cost (register footprint), plus threading the clusters:
///   - `jit_single_fused`  : whole dashboard in ONE loop (most live xmm).
///   - `jit_split_seq`     : one loop per group, run sequentially.
///   - `jit_split_parallel`: groups on separate host threads (bulk).
/// Hand is the per-indicator independent passes (the existing baselines); here we
/// compare single-vs-split *within the engine*, which is the grouping question.
fn dashboard() -> Vec<(Recipe, Option<Vec<bool>>)> {
    vec![
        (Apo::recipe(&APO), None),
        (Macd::recipe(&MACD), None),
        (Ppo::recipe(&PPO), None),
        (Ema::recipe(&[9.0]), None),
        (Atr::recipe(&[14.0]), None),
        (Natr::recipe(&[14.0]), None),
        (SuperTrend::recipe(&[14.0, 3.0]), None),
    ]
}

fn bench_grouping(c: &mut Criterion) {
    let (hi, lo, cl) = hlc(0);
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let groups = {
        let mut r = Request::new();
        for (rec, opt) in dashboard() {
            r = r.add(rec, opt.as_deref());
        }
        r.sharing_groups()
    };
    eprintln!("grouping: {} groups = {:?}", groups.len(), groups);

    let mut g = c.benchmark_group("grouping");
    g.sample_size(20);

    // one fused loop over the whole dashboard.
    let single = {
        let mut r = Request::new();
        for (rec, opt) in dashboard() {
            r = r.add(rec, opt.as_deref());
        }
        r.build()
    };
    g.bench_function("jit_single_fused", |b| {
        b.iter(|| {
            let (res, qs) = single.run(&three);
            black_box((res.outputs[0][0].len(), qs.next_bar()))
        })
    });

    // one plan per sharing group, run sequentially.
    let split: Vec<JitPlan> = groups
        .iter()
        .map(|idxs| {
            let mut r = Request::new();
            for &i in idxs {
                let (rec, opt) = dashboard()[i].clone();
                r = r.add(rec, opt.as_deref());
            }
            r.build()
        })
        .collect();
    g.bench_function("jit_split_seq", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in &split {
                let ns = p.n_streams();
                let (res, _) = p.run(&three[..ns]);
                acc += res.outputs[0][0].len();
            }
            black_box(acc)
        })
    });

    // groups on separate host threads (bulk run; needs `Results: Send`).
    g.bench_function("jit_split_parallel", |b| {
        b.iter(|| {
            std::thread::scope(|s| {
                let mut acc = 0usize;
                for p in &split {
                    let ns = p.n_streams();
                    s.spawn(move || {
                        let (res, _) = p.run(&three[..ns]);
                        black_box(res.outputs[0][0].len());
                    });
                }
                black_box(&mut acc);
            })
        })
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
    bench_rolling,
    bench_rolling_hma,
    bench_rolling_hma_multi,
    bench_hma_group_dedup,
    bench_rolling_wilders,
    bench_rolling_atr,
    bench_rolling_adx,
    bench_rolling_st,
    bench_multi_input,
    bench_rolling_mixed,
    bench_tick,
    bench_grouping
);
criterion_main!(benches);
