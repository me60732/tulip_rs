use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::di::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_di, ti_di_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_minus_di, ta_minus_di_start, ta_plus_di, ta_plus_di_start};

// Test data from di_test.rs
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

// Options from di_test.rs (using single period option for simplicity)
const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

// Chunk size for from_state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for synthetic benchmarking
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..499 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
    }
    (high_vec, low_vec, close_vec)
}

/// Extract HLC arrays from stock data
fn get_hlc_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

fn bench_c_di(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = unsafe { ti_di_start(options.as_ptr()) };
                        let output_len = high.len() - (start_index as usize);
                        let mut plus_di_vec = vec![0.0_f64; output_len];
                        let mut minus_di_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> =
                            vec![plus_di_vec.as_mut_ptr(), minus_di_vec.as_mut_ptr()];
                        let ret = unsafe {
                            ti_di(
                                high.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_di returned error code {}", ret);
                        black_box(&plus_di_vec);
                        black_box(&minus_di_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("di", "C_tulip", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("C DI {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                    let start_index = unsafe { ti_di_start(options.as_ptr()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut plus_di_vec = vec![0.0_f64; output_len];
                    let mut minus_di_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> =
                        vec![plus_di_vec.as_mut_ptr(), minus_di_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_di(
                            high.len() as i32,
                            black_box(&inputs).as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_di returned error code {}", ret);
                    black_box(&plus_di_vec);
                    black_box(&minus_di_vec);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_di(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("DI indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("di", "Rust", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("Rust DI {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                    let result = indicator(&inputs, &options, None).expect("DI indicator failed");
                    black_box(&result);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_di_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [
                            &high[..min_data_val],
                            &low[..min_data_val],
                            &close[..min_data_val],
                        ];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("DI Indicator Failed");

                        // Chunks
                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for ((high_chunk, low_chunk), close_chunk) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let chunk_inputs = [high_chunk, low_chunk, close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        let close_rem = close_chunks.remainder();

                        if !high_rem.is_empty() {
                            let chunk_inputs = [high_rem, low_rem, close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "di",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [
                        &high[..high.len() - 1],
                        &low[..low.len() - 1],
                        &close[..close.len() - 1],
                    ];
                    let final_inputs = [
                        &high[high.len() - 1..],
                        &low[low.len() - 1..],
                        &close[close.len() - 1..],
                    ];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust DI indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust DI from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "di",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust DI indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust DI from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "di",
                        "Rust_FromState_1_Bar_json",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );
                }
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(&format!("Rust DI from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                //let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("DI indicator failed");

                    // Chunks
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), close_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_inputs = [high_chunk, low_chunk, close_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the TA-Lib implementation of DI (using PLUS_DI and MINUS_DI).
#[cfg(feature = "talib")]
fn bench_talib_di(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index_plus = ta_plus_di_start(options[0]);
                        let start_index_minus = ta_minus_di_start(options[0]);
                        let start_index = start_index_plus.max(start_index_minus);
                        assert!(start_index >= 0, "ta_di_start returned a negative index");
                        let output_len = high.len() - (start_index as usize);
                        let mut plus_di_vec = vec![0.0_f64; output_len];
                        let mut minus_di_vec = vec![0.0_f64; output_len];
                        let mut plus_outputs: Vec<*mut f64> = vec![plus_di_vec.as_mut_ptr()];
                        let mut minus_outputs: Vec<*mut f64> = vec![minus_di_vec.as_mut_ptr()];

                        let ret_plus = ta_plus_di(
                            high.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            plus_outputs.as_mut_ptr(),
                        );
                        let ret_minus = ta_minus_di(
                            high.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            minus_outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret_plus, 0, "ta_plus_di returned error code {}", ret_plus);
                        assert_eq!(
                            ret_minus, 0,
                            "ta_minus_di returned error code {}",
                            ret_minus
                        );
                        black_box(&plus_di_vec);
                        black_box(&minus_di_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("di", "talib", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index_plus = ta_plus_di_start(options[0]);
            let start_index_minus = ta_minus_di_start(options[0]);
            let start_index = start_index_plus.max(start_index_minus);
            assert!(start_index >= 0, "ta_di_start returned a negative index");
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("di_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("TA-Lib DI {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut plus_di_vec = vec![0.0_f64; output_len];
                    let mut minus_di_vec = vec![0.0_f64; output_len];
                    let mut plus_outputs: Vec<*mut f64> = vec![plus_di_vec.as_mut_ptr()];
                    let mut minus_outputs: Vec<*mut f64> = vec![minus_di_vec.as_mut_ptr()];

                    let ret_plus = ta_plus_di(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        plus_outputs.as_mut_ptr(),
                    );
                    let ret_minus = ta_minus_di(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        minus_outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret_plus, 0, "ta_plus_di returned error code {}", ret_plus);
                    assert_eq!(
                        ret_minus, 0,
                        "ta_minus_di returned error code {}",
                        ret_minus
                    );
                    black_box(&plus_di_vec);
                    black_box(&minus_di_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of DI with optional outputs.
fn bench_rust_di_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust DI indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "di",
                    "Rust_optional",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("di_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust DI {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &options, Some(&[true, true]))
                        .expect("Rust DI indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_di_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close) = get_hlc_arrays(data);
                (symbol.clone(), high, low, close)
            })
            .collect();

        // Test each period
        for options in &OPTIONS_LIST {
            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                    &stock_data[0].3, // close
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                    &stock_data[1].3, // close
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                    &stock_data[2].3, // close
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                    &stock_data[3].3, // close
                ],
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, options, None)
                        .expect("Rust SIMD by assets DI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "di",
                "Rust_SIMD_by_assets",
                options,
                stock_data[0].1.len(),
                &timing,
                Some(&format!("4_Assets_Period_{}", options[0])),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();

        // Test each period
        for options in &OPTIONS_LIST {
            // Create 4 identical datasets for SIMD processing
            let inputs: [&[&[f64]; 3]; 4] = [
                &[&high_vec, &low_vec, &close_vec],
                &[&high_vec, &low_vec, &close_vec],
                &[&high_vec, &low_vec, &close_vec],
                &[&high_vec, &low_vec, &close_vec],
            ];

            let mut group = c.benchmark_group("di_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets DI period {}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, options, None)
                            .expect("Rust SIMD by assets DI indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_di_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("di");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high_vec, low_vec, close_vec) = get_hlc_arrays(&stock_data);
            let inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process all 4 options with 4-wide SIMD
                    let options_4 = [
                        &OPTIONS_LIST[0],
                        &OPTIONS_LIST[1],
                        &OPTIONS_LIST[2],
                        &OPTIONS_LIST[3],
                    ];
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD DI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "di",
                "Rust_SIMD",
                &[0.0],
                high_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("di_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options DI (4 lanes)", |b| {
            b.iter(|| {
                // Process all 4 options with 4-wide SIMD
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD DI indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_di_simd_by_options,
    bench_rust_di_simd_by_assets,
    bench_rust_di,
    bench_c_di,
    bench_talib_di,
    bench_rust_di_optional,
    bench_rust_di_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_di_simd_by_options,
    bench_rust_di_simd_by_assets,
    bench_rust_di,
    bench_c_di,
    bench_rust_di_optional,
    bench_rust_di_from_state,
);
criterion_main!(benches);
