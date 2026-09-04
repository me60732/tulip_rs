use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicator_types::TIndicatorState;
use tulip_rs::indicators::mama::{Indicator, IndicatorByOptions, Mama};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_mama, ta_mama_start};

const SAMPLE_SIZE: usize = 10000;
const CHUNK_SIZE: usize = 100;

const OPTIONS_4: [[f64; 2]; 4] = [[0.5, 0.05], [0.4, 0.04], [0.3, 0.03], [0.2, 0.02]];

const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

fn expand_inputs() -> Vec<f64> {
    let mut v = CLOSE.to_vec();
    for _ in 0..499 {
        v.extend_from_slice(&CLOSE);
    }
    v // ~7500 bars
}

fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

/// Full run over ~7500 bars — cycles through all four option sets.
fn bench_mama(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("mama");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];
            for opts in &OPTIONS_4 {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let r = Mama::indicator(&inputs, opts, None).expect("MAMA failed");
                        black_box(&r);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("mama", "Rust", opts, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("mama_rust");
        group.sample_size(SAMPLE_SIZE);
        for opts in &OPTIONS_4 {
            let label = format!(
                "Rust MAMA/FAMA full run [fast={}, slow={}]",
                opts[0], opts[1]
            );
            group.bench_function(&label, |b| {
                b.iter(|| black_box(Mama::indicator(&inputs, opts, None).expect("MAMA failed")));
            });
        }
        group.finish();
    }
}

/// Streaming from saved state: chunked + single-bar variants — cycles through all four option sets.
fn bench_mama_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("mama");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            for opts in &OPTIONS_4 {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let seed = Mama::min_data(opts).max(CHUNK_SIZE);
                        let (_, mut state) = Mama::indicator(&[&close[..seed]], opts, None)
                            .expect("MAMA seed failed");
                        for chunk in close[seed..].chunks_exact(CHUNK_SIZE) {
                            black_box(state.batch_indicator(&[chunk], None).expect("batch failed"));
                        }
                        let rem = &close[seed + (close[seed..].len() / CHUNK_SIZE) * CHUNK_SIZE..];
                        if !rem.is_empty() {
                            black_box(state.batch_indicator(&[rem], None).expect("batch failed"));
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "mama",
                    "Rust_FromState",
                    opts,
                    n,
                    &timing,
                    Some(stock_symbol.as_str()),
                );

                if n > 1 {
                    let (_, mut state) = Mama::indicator(&[&close[..n - 1]], opts, None)
                        .expect("MAMA 1-bar seed failed");
                    let final_input = [&close[n - 1..]];
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            black_box(
                                state
                                    .batch_indicator(&final_input, None)
                                    .expect("1-bar failed"),
                            );
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "mama",
                        "Rust_FromState_1_Bar",
                        opts,
                        n,
                        &timing,
                        Some(stock_symbol.as_str()),
                    );
                }
            }
        }
    } else {
        let close = expand_inputs();
        let mut group = c.benchmark_group("mama_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        for opts in &OPTIONS_4 {
            let seed = Mama::min_data(opts).max(CHUNK_SIZE);
            let (_, mut state) =
                Mama::indicator(&[&close[..seed]], opts, None).expect("MAMA seed failed");
            let label = format!(
                "Rust MAMA/FAMA from state [fast={}, slow={}]",
                opts[0], opts[1]
            );
            group.bench_function(&label, |b| {
                b.iter(|| {
                    for chunk in close[seed..].chunks_exact(CHUNK_SIZE) {
                        black_box(state.batch_indicator(&[chunk], None).expect("batch failed"));
                    }
                });
            });
        }
        group.finish();

        if close.len() > 1 {
            let mut group = c.benchmark_group("mama_rust_from_state_1_bar");
            group.sample_size(SAMPLE_SIZE);
            for opts in &OPTIONS_4 {
                let (_, mut state) = Mama::indicator(&[&close[..close.len() - 1]], opts, None)
                    .expect("MAMA 1-bar seed failed");
                let final_input = [&close[close.len() - 1..]];
                let label = format!(
                    "Rust MAMA/FAMA from state 1 bar [fast={}, slow={}]",
                    opts[0], opts[1]
                );
                group.bench_function(&label, |b| {
                    b.iter(|| {
                        black_box(
                            state
                                .batch_indicator(&final_input, None)
                                .expect("1-bar failed"),
                        )
                    });
                });
            }
            group.finish();
        }
    }
}

/// SIMD by_assets: 4 assets processed simultaneously — cycles through all four option sets.
fn bench_mama_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("mama");
        let data = get_all_stock_data().unwrap();
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];
        for opts in &OPTIONS_4 {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let r = Mama::indicator_by_assets::<4>(&inputs, opts, None)
                        .expect("SIMD by_assets failed");
                    black_box(&r);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "mama",
                "Rust_SIMD_by_assets",
                opts,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let close = expand_inputs();
        let inputs: [&[&[f64]; 1]; 4] = [&[&close], &[&close], &[&close], &[&close]];
        let mut group = c.benchmark_group("mama_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        for opts in &OPTIONS_4 {
            let label = format!(
                "Rust SIMD by_assets MAMA/FAMA (N=4) [fast={}, slow={}]",
                opts[0], opts[1]
            );
            group.bench_function(&label, |b| {
                b.iter(|| {
                    black_box(
                        Mama::indicator_by_assets::<4>(&inputs, opts, None)
                            .expect("SIMD by_assets failed"),
                    )
                });
            });
        }
        group.finish();
    }
}

/// SIMD by_options: all 4 option sets on one asset simultaneously.
fn bench_mama_simd_by_options(c: &mut Criterion) {
    let options_refs: [&[f64; 2]; 4] = [&OPTIONS_4[0], &OPTIONS_4[1], &OPTIONS_4[2], &OPTIONS_4[3]];

    if should_log_to_db() {
        init_database_data();
        init_logging("mama");
        let data = get_all_stock_data().unwrap();
        let (stock_symbol, stock_data) = data.iter().next().unwrap();
        let close = get_close_array(stock_data);
        let inputs = [close.as_slice()];
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let r = Mama::indicator_by_options::<4>(&inputs, &options_refs, None)
                    .expect("SIMD by_options failed");
                black_box(&r);
            },
            SAMPLE_SIZE,
        );
        // Log once per OPTIONS_4 entry so each option set is recorded.
        for opts in &OPTIONS_4 {
            log_timing_result(
                "mama",
                "Rust_SIMD_by_options",
                opts,
                close.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("mama_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function(
            "Rust SIMD by_options MAMA/FAMA (N=4, all option sets)",
            |b| {
                b.iter(|| {
                    black_box(
                        Mama::indicator_by_options::<4>(&inputs, &options_refs, None)
                            .expect("SIMD by_options failed"),
                    )
                });
            },
        );
        group.finish();
    }
}

/// TA-Lib MAMA/FAMA — cycles through all four option sets.
#[cfg(feature = "talib")]
fn bench_talib_mama(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("mama");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];
            for opts in &OPTIONS_4 {
                let lookback = ta_mama_start(opts[0], opts[1]);
                assert!(lookback >= 0, "ta_mama_start returned negative");
                let out_len = n - lookback as usize;
                let mut out_mama = vec![0.0_f64; out_len];
                let mut out_fama = vec![0.0_f64; out_len];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut outputs: Vec<*mut f64> =
                            vec![out_mama.as_mut_ptr(), out_fama.as_mut_ptr()];
                        let ret = ta_mama(
                            n as i32,
                            inputs.as_ptr(),
                            opts.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_mama returned error {ret}");
                        black_box(&out_mama);
                        black_box(&out_fama);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("mama", "talib", opts, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close = expand_inputs();
        let n = close.len();
        let inputs: Vec<*const f64> = vec![close.as_ptr()];
        let mut group = c.benchmark_group("mama_talib");
        group.sample_size(SAMPLE_SIZE);
        for opts in &OPTIONS_4 {
            let lookback = ta_mama_start(opts[0], opts[1]);
            assert!(lookback >= 0, "ta_mama_start returned negative");
            let out_len = n - lookback as usize;
            let label = format!("TA-Lib MAMA/FAMA [fast={}, slow={}]", opts[0], opts[1]);
            group.bench_function(&label, |b| {
                b.iter(|| {
                    let mut out_mama = vec![0.0_f64; out_len];
                    let mut out_fama = vec![0.0_f64; out_len];
                    let mut outputs: Vec<*mut f64> =
                        vec![out_mama.as_mut_ptr(), out_fama.as_mut_ptr()];
                    let ret = ta_mama(
                        n as i32,
                        inputs.as_ptr(),
                        opts.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_mama returned error {ret}");
                    black_box(&out_mama);
                    black_box(&out_fama);
                });
            });
        }
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_mama_simd_by_assets,
    bench_mama_simd_by_options,
    bench_mama,
    bench_talib_mama,
    bench_mama_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_mama_simd_by_assets,
    bench_mama_simd_by_options,
    bench_mama,
    bench_mama_from_state,
);
criterion_main!(benches);
