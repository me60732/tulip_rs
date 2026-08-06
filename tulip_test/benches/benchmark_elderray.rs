use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::elderray::{Elderray, Indicator, TIndicatorState, IndicatorState};
use tulip_rs::indicators::elderray::indicator_by_assets;
use tulip_rs::indicators::elderray::indicator_by_options;
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_ema, ti_ema_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample OHLC input data
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options: [period]
const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

// Chunk size for batched (from-state) processing
const CHUNK_SIZE: usize = 100;

/// Expand sample data by repetition to create a realistic dataset.
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
    }
    (high_vec, low_vec, close_vec)
}

fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

/// Benchmark the C Tulip equivalent of Elder-ray.
///
/// There is no native `ti_elderray`, so this runs `ti_ema(close, period)` then
/// folds a bull/bear pass over the result — exactly the work the Rust indicator
/// does internally.
fn bench_c_elderray(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = close.len();
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start = unsafe { ti_ema_start(options.as_ptr()) } as usize;
                        let output_len = n - start;
                        let mut ema_c = vec![0.0_f64; output_len];
                        let mut outputs_c: Vec<*mut f64> = vec![ema_c.as_mut_ptr()];

                        unsafe {
                            ti_ema(
                                n as i32,
                                inputs_c.as_ptr(),
                                options.as_ptr(),
                                outputs_c.as_mut_ptr(),
                            );
                        }

                        for i in 0..output_len {
                            let bull = high[start + i] - ema_c[i];
                            let bear = low[start + i] - ema_c[i];
                            black_box((bull, bear));
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "elderray",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let n = close_vec.len();
        let inputs_c: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start = unsafe { ti_ema_start(options.as_ptr()) } as usize;
            let output_len = n - start;

            let mut group = c.benchmark_group("elderray_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C Elder-ray {{ period: {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut ema_c = vec![0.0_f64; output_len];
                    let mut outputs_c: Vec<*mut f64> = vec![ema_c.as_mut_ptr()];

                    unsafe {
                        ti_ema(
                            n as i32,
                            inputs_c.as_ptr(),
                            options.as_ptr(),
                            outputs_c.as_mut_ptr(),
                        );
                    }

                    for i in 0..output_len {
                        let bull = high_vec[start + i] - ema_c[i];
                        let bear = low_vec[start + i] - ema_c[i];
                        black_box((bull, bear));
                    }
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of Elder-ray.
fn bench_rust_elderray(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = close.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = Elderray::indicator(&inputs, &options, None)
                            .expect("Rust Elder-ray failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("elderray", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("elderray_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust Elder-ray {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = Elderray::indicator(&inputs, &options, None)
                            .expect("Rust Elder-ray failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of Elder-ray with the optional EMA output enabled.
fn bench_rust_elderray_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = close.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = Elderray::indicator(&inputs, &options, Some(&[true]))
                            .expect("Rust Elder-ray optional failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "elderray",
                    "Rust_optional",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("elderray_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust Elder-ray optional {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = Elderray::indicator(&inputs, &options, Some(&[true]))
                            .expect("Rust Elder-ray optional failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust streaming (from-state) implementation of Elder-ray.
fn bench_rust_elderray_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                // --- Rust_FromState (chunked) ---
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min = Elderray::min_data(&options);
                        let chunk_inputs = [&high[..min], &low[..min], &close[..min]];
                        let (_, mut state) = Elderray::indicator(&chunk_inputs, &options, None)
                            .expect("Elder-ray indicator failed");

                        let mut high_chunks = high[min..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close[min..].chunks_exact(CHUNK_SIZE);

                        for ((hc, lc), cc) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let result = state.batch_indicator(&[hc, lc, cc], None);
                            black_box(&result);
                        }

                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        let close_rem = close_chunks.remainder();
                        if !high_rem.is_empty() {
                            let result =
                                state.batch_indicator(&[high_rem, low_rem, close_rem], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "elderray",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                if n > 1 {
                    let new_inputs = [&high[..n - 1], &low[..n - 1], &close[..n - 1]];
                    let final_inputs = [&high[n - 1..], &low[n - 1..], &close[n - 1..]];

                    // --- Rust_FromState_1_Bar ---
                    let (_, mut state) =
                        Elderray::indicator(&new_inputs, &options, None).expect("Elder-ray indicator failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Elder-ray from-state failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "elderray",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json ---
                    let (_, state) =
                        Elderray::indicator(&new_inputs, &options, None).expect("Elder-ray indicator failed");
                    let json = serde_json::to_string(&state).expect("JSON serialisation failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON deserialisation failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Elder-ray from-state failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "elderray",
                        "Rust_FromState_1_Bar_json",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );
                }
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let min = Elderray::min_data(&options);
            let chunk_inputs = [&high_vec[..min], &low_vec[..min], &close_vec[..min]];
            let (_, mut state) =
                Elderray::indicator(&chunk_inputs, &options, None).expect("Elder-ray indicator failed");

            let mut group = c.benchmark_group("elderray_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust Elder-ray from state {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let mut high_chunks = high_vec[min..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low_vec[min..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close_vec[min..].chunks_exact(CHUNK_SIZE);

                        for ((hc, lc), cc) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let result = state.batch_indicator(&[hc, lc, cc], None);
                            black_box(&result);
                        }

                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        let close_rem = close_chunks.remainder();
                        if !high_rem.is_empty() {
                            let result =
                                state.batch_indicator(&[high_rem, low_rem, close_rem], None);
                            black_box(&result);
                        }
                    });
                },
            );
            group.finish();

            if high_vec.len() > 1 {
                let n = high_vec.len();
                let new_inputs = [&high_vec[..n - 1], &low_vec[..n - 1], &close_vec[..n - 1]];
                let final_inputs = [&high_vec[n - 1..], &low_vec[n - 1..], &close_vec[n - 1..]];
                let (_, mut state) =
                    Elderray::indicator(&new_inputs, &options, None).expect("Elder-ray indicator failed");

                let mut group = c.benchmark_group("elderray_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!(
                        "Rust Elder-ray from state 1 bar {{ period: {} }}",
                        options[0]
                    ),
                    |b| {
                        b.iter(|| {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Elder-ray from-state failed");
                            black_box(&result);
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by-assets implementation of Elder-ray.
///
/// Processes 4 assets simultaneously with shared options.
fn bench_rust_elderray_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
        let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
        let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
        let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
        let inputs: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("SIMD by-assets Elder-ray failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "elderray",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();

        let asset: [&[f64]; 3] = [&high_vec, &low_vec, &close_vec];
        let inputs: [&[&[f64]; 3]; 4] = [&asset, &asset, &asset, &asset];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("elderray_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust SIMD by assets Elder-ray {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by-assets Elder-ray failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by-options implementation of Elder-ray.
///
/// Processes all 4 option sets on a single asset in one SIMD-accelerated pass.
fn bench_rust_elderray_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("elderray");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let n = high.len();
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("SIMD by-options Elder-ray failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "elderray",
                "Rust_SIMD",
                &[0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("elderray_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options Elder-ray (4 lanes)", |b| {
            b.iter(|| {
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by-options Elder-ray failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_elderray_simd_by_assets,
    bench_rust_elderray_simd_by_options,
    bench_rust_elderray,
    bench_c_elderray,
    bench_rust_elderray_optional,
    bench_rust_elderray_from_state,
);
criterion_main!(benches);
