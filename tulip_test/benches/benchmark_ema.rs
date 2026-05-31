use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tulip_rs::indicators::ema::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
//const SAMPLE_SIZE: usize = 1000000;
use tulip_test::c_bindings::{ti_ema, ti_ema_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ema, ta_ema_start};

// Sample input data (close prices)
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for EMA (period)
//const OPTIONS_LIST: [[f64; 1]; 8] = [[5.0], [12.0], [14.0], [20.0], [26.0], [50.0], [100.0], [200.0]];
//const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [12.0], [14.0], [20.0], [26.0], [50.0]];
const OPTIONS_LIST: [[f64; 1]; 4] = [[14.0], [20.0], [26.0], [50.0]];

// Chunk size for batched processing
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it.
fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..500 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

// Helper function to get close array from stock data
fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

/// Benchmark the C implementation of EMA.
fn bench_c_ema(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = unsafe { ti_ema_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_ema_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = unsafe {
                            ti_ema(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_ema returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("ema", "C_tulip", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_ema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_ema_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("ema_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C EMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_ema(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_ema returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of EMA.
fn bench_rust_ema(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        indicator(&inputs, &options, None).expect("Rust EMA indicator failed");
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ema",
                    "Rust",
                    &options,
                    close.len(),
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("ema_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust EMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    indicator(&inputs, &options, None).expect("Rust EMA indicator failed");
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of EMA.
fn bench_rust_ema_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options);
                        // First chunk
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("EMA indicator failed");

                        // Chunks
                        let mut close_chunks = close[min_data..].chunks_exact(CHUNK_SIZE);

                        for close_chunk in close_chunks.by_ref() {
                            let chunk_inputs = [close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
                        let close_rem = close_chunks.remainder();

                        if !close_rem.is_empty() {
                            let chunk_inputs = [close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ema",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [&close[..close.len() - 1]];
                    let final_inputs = [&close[close.len() - 1..]];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust EMA indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust EMA from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ema",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust EMA indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust EMA from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ema",
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
        // Criterion profiling mode - benchmark synthetic data
        let close_vec = expand_inputs();
        let _inputs = [&close_vec];

        for options in OPTIONS_LIST {
            let min_data = min_data(&options);
            // First chunk
            let chunk_inputs = [&close_vec[..min_data]];

            let (_, mut state) =
                indicator(&chunk_inputs, &options, None).expect("EMA indicator failed");

            let mut group = c.benchmark_group("ema_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust EMA from state {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);

                    for close_chunk in close_chunks.by_ref() {
                        let chunk_inputs = [close_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let close_rem = close_chunks.remainder();

                    if !close_rem.is_empty() {
                        let chunk_inputs = [close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_inputs = [&close_vec[..close_vec.len() - 1]];
                let final_inputs = [&close_vec[close_vec.len() - 1..]];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust EMA indicator failed");

                let mut group = c.benchmark_group("ema_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!("Rust EMA from state 1 bar {{ {} }}", options[0]),
                    |b| {
                        b.iter(|| {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust EMA from state indicator failed");
                            black_box(&result);
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark the TA-Lib implementation of EMA.
#[cfg(feature = "talib")]
fn bench_talib_ema(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_ema_start(options[0]);
                        assert!(start_index >= 0, "ta_ema_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_ema(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_ema returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("ema", "talib", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_ema_start(options[0]);
            assert!(start_index >= 0, "ta_ema_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("ema_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("TA-Lib EMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_ema(
                        close_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_ema returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of EMA.
fn bench_rust_ema_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets EMA indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ema",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("All"), //Some(&stock_names),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 1]; 4] = [
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("ema_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust SIMD by assets EMA {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets EMA indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD implementation of EMA.
//#[cfg(feature = "portable_simd")]
fn bench_rust_ema_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process 4 options with 4-wide SIMD
                    let options_4 = [
                        &OPTIONS_LIST[0],
                        &OPTIONS_LIST[1],
                        &OPTIONS_LIST[2],
                        &OPTIONS_LIST[3],
                    ];
                    let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD EMA indicator failed");
                    black_box(&result_4);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("ema", "Rust_SIMD", &[0.0], n, &timing, Some(stock_symbol));
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("ema_rust_simd");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD EMA (4 lanes)", |b| {
            b.iter(|| {
                // Process 4 options with 4-wide SIMD
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD EMA indicator failed");
                black_box(&result_4);
            });
        });
        group.finish();
    }
}

/// Benchmark the `ta` crate (RustTa) implementation of EMA.
fn bench_rust_ta_ema(c: &mut Criterion) {
    use ta::indicators::ExponentialMovingAverage;
    use ta::Next;

    if should_log_to_db() {
        init_database_data();
        init_logging("ema");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                let period = options[0] as usize;
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut ema =
                            ExponentialMovingAverage::new(period).expect("ta EMA new failed");
                        let mut last = 0.0_f64;
                        for &price in &close {
                            last = ema.next(price);
                        }
                        black_box(last);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("ema", "RustTa", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let period = options[0] as usize;
            let mut group = c.benchmark_group("ema_rust_ta");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("RustTa EMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut ema = ExponentialMovingAverage::new(period).expect("ta EMA new failed");
                    let mut last = 0.0_f64;
                    for &price in &close_vec {
                        last = ema.next(price);
                    }
                    black_box(last);
                });
            });
            group.finish();
        }
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_ema_simd_by_options,
    bench_rust_ema_simd_by_assets,
    bench_rust_ema,
    bench_rust_ta_ema,
    bench_c_ema,
    bench_rust_ema_from_state,
    bench_talib_ema,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_ema_simd_by_options,
    bench_rust_ema_simd_by_assets,
    bench_rust_ema,
    bench_rust_ta_ema,
    bench_c_ema,
    bench_rust_ema_from_state,
);
criterion_main!(benches);
