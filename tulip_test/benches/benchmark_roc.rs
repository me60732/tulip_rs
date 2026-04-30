use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::roc::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_roc, ti_roc_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
use tulip_test::talib_bindings::{ta_roc, ta_roc_start};

// Sample input data (close prices)
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for ROC (period)
//const OPTIONS_LIST: [[f64; 1]; 5] = [[5.0], [10.0], [14.0], [20.0], [25.0]];
const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    [10.0],
    [14.0],
    [20.0],
    [25.0],
    [30.0],
    [50.0],
    [100.0],
];
/// Chunk size for from-state benchmarks
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

/// Benchmark the C implementation of ROC.
fn bench_c_roc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_roc_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_roc_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_roc(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_roc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("roc", "C_tulip", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_roc_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_roc_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("roc_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("C ROC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_roc(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_roc returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of ROC.
fn bench_rust_roc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("ROC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("roc", "Rust", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("roc_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust ROC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust ROC indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of ROC.
fn bench_rust_roc_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("ROC indicator failed");

                        // Chunks
                        let mut close_chunks = close[min_data..].chunks_exact(CHUNK_SIZE);

                        for close_chunk in close_chunks.by_ref() {
                            let result = state.batch_indicator(&[close_chunk], None);
                            black_box(&result);
                        }

                        // Remainder
                        let close_rem = close_chunks.remainder();

                        if !close_rem.is_empty() {
                            let result = state.batch_indicator(&[close_rem], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "roc",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_close = close[..close.len() - 1].to_vec();
                    let final_close = close[close.len() - 1..].to_vec();
                    let new_inputs = [new_close.as_slice()];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust ROC indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&[final_close.as_slice()], None)
                                .expect("Rust ROC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "roc",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust ROC indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&[final_close.as_slice()], None)
                                .expect("Rust ROC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "roc",
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
        let close_vec = expand_inputs();
        let _inputs = [&close_vec];

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(&format!("Rust ROC from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("ROC indicator failed");

                    // Chunks
                    let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);

                    for close_chunk in close_chunks.by_ref() {
                        let result = state.batch_indicator(&[close_chunk], None);
                        black_box(&result);
                    }

                    // Remainder
                    let close_rem = close_chunks.remainder();

                    if !close_rem.is_empty() {
                        let result = state.batch_indicator(&[close_rem], None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_close = close_vec[..close_vec.len() - 1].to_vec();
                let final_close = close_vec[close_vec.len() - 1..].to_vec();
                let new_inputs = [new_close.as_slice()];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust ROC indicator failed");

                let mut group = c.benchmark_group(&format!(
                    "Rust ROC from state 1 bar {{ {:.1} }}",
                    options[0]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&[final_close.as_slice()], None)
                            .expect("Rust ROC from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust implementation of ROC with optional outputs.
fn bench_rust_roc_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust ROC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "roc",
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
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("roc_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust ROC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &options, Some(&[true, true]))
                        .expect("Rust ROC indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the TA-Lib implementation of ROC.
fn bench_talib_roc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_roc_start(options[0]);
                        assert!(start_index >= 0, "ta_roc_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_roc(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_roc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("roc", "talib", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_roc_start(options[0]);
            assert!(start_index >= 0, "ta_roc_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("roc_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("TA-Lib ROC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_roc(
                        close_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_roc returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of ROC.
fn bench_rust_roc_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            // Test without optional outputs
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets ROC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "roc",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("All"),
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
            // Benchmark without optional outputs
            let mut group = c.benchmark_group("roc_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets ROC {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets ROC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();

            // Benchmark with optional outputs
            let mut group = c.benchmark_group("roc_rust_simd_by_assets_optional");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets ROC optional {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result =
                            indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                                .expect(
                                "Rust SIMD by assets ROC indicator with optional outputs failed",
                            );
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_roc_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    let options_4_second = [
        &OPTIONS_LIST[4],
        &OPTIONS_LIST[5],
        &OPTIONS_LIST[6],
        &OPTIONS_LIST[7],
    ];
    if should_log_to_db() {
        init_database_data();
        init_logging("roc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [close_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process first 4 options with 4-wide SIMD
                    let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD ROC indicator failed");
                    black_box(&result_4);

                    // Process remaining 4 options with 4-wide SIMD
                    let result_4_second =
                        indicator_by_options::<4>(&inputs, &options_4_second, None)
                            .expect("Rust SIMD ROC indicator failed");
                    black_box(&result_4_second);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "roc",
                "Rust_SIMD",
                &[0.0],
                close_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("roc_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options ROC (4+4+2 lanes)", |b| {
            b.iter(|| {
                // Process first 4 options with 4-wide SIMD
                let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD ROC indicator failed");
                black_box(&result_4);

                let result_4_second = indicator_by_options::<4>(&inputs, &options_4_second, None)
                    .expect("Rust SIMD ROC indicator failed");
                black_box(&result_4_second);
            });
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_roc_simd_by_assets,
    bench_rust_roc_simd_by_options,
    bench_rust_roc,
    bench_c_roc,
    bench_talib_roc,
    bench_rust_roc_from_state,
    bench_rust_roc_optional,
);
criterion_main!(benches);
