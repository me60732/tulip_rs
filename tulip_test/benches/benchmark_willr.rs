use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::willr::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_willr, ti_willr_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_willr, ta_willr_start};

// Sample input data from willr_test.rs
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

// Options for WILLR (period)
/*const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    [7.0],
    [10.0],
    [14.0],
    [20.0],
    [25.0],
    [50.0],
    [100.0],
];*/
/*const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    [10.0],
    [14.0],
    [20.0],
    [25.0],
    [35.0],
    [50.0],
    [100.0],
];*/
const OPTIONS_LIST: [[f64; 1]; 4] = [[25.0], [35.0], [50.0], [100.0]];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

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

/// Benchmark the C implementation of WILLR.
fn bench_c_willr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs: Vec<*const f64> =
                vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_willr_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_willr_start returned a negative index");
                        let output_len = high_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_willr(
                                high_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_willr returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "willr",
                    "C_tulip",
                    &options,
                    high_vec.len(),
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_willr_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_willr_start returned a negative index");
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("willr_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C WILLR {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_willr(
                            high_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_willr returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of WILLR.
fn bench_rust_willr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust WILLR indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "willr",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("willr_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust WILLR {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust WILLR indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of WILLR.
fn bench_rust_willr_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = high_vec.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [
                            &high_vec[..min_data_val],
                            &low_vec[..min_data_val],
                            &close_vec[..min_data_val],
                        ];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("WILLR indicator failed");

                        // Chunks
                        let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

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

                        if !high_rem.is_empty() && !low_rem.is_empty() && !close_rem.is_empty() {
                            let chunk_inputs = [high_rem, low_rem, close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "willr",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if high_vec.len() > 1 {
                    let new_high_vec = high_vec[..high_vec.len() - 1].to_vec();
                    let new_low_vec = low_vec[..low_vec.len() - 1].to_vec();
                    let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                    let new_inputs = [
                        new_high_vec.as_slice(),
                        new_low_vec.as_slice(),
                        new_close_vec.as_slice(),
                    ];
                    let final_high_vec = high_vec[high_vec.len() - 1..].to_vec();
                    let final_low_vec = low_vec[low_vec.len() - 1..].to_vec();
                    let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust WILLR indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(
                                    &[&final_high_vec, &final_low_vec, &final_close_vec],
                                    None,
                                )
                                .expect("Rust WILLR from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "willr",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust WILLR indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(
                                    &[&final_high_vec, &final_low_vec, &final_close_vec],
                                    None,
                                )
                                .expect("Rust WILLR from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "willr",
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
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let _inputs = [&high_vec, &low_vec, &close_vec];

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(format!("Rust WILLR from state {{ {} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("WILLR indicator failed");

                    // Chunks
                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

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

                    if !high_rem.is_empty() && !low_rem.is_empty() && !close_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if high_vec.len() > 1 {
                let new_high_vec = high_vec[..high_vec.len() - 1].to_vec();
                let new_low_vec = low_vec[..low_vec.len() - 1].to_vec();
                let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                let new_inputs = [
                    new_high_vec.as_slice(),
                    new_low_vec.as_slice(),
                    new_close_vec.as_slice(),
                ];

                let final_high_vec = high_vec[high_vec.len() - 1..].to_vec();
                let final_low_vec = low_vec[low_vec.len() - 1..].to_vec();
                let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust WILLR indicator failed");

                let mut group =
                    c.benchmark_group(format!("Rust WILLR from state 1 bar {{ {} }}", options[0]));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(
                                &[&final_high_vec, &final_low_vec, &final_close_vec],
                                None,
                            )
                            .expect("Rust WILLR from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the TA-Lib implementation of WILLR.
#[cfg(feature = "talib")]
fn bench_talib_willr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_willr_start(options[0]);
                        assert!(start_index >= 0, "ta_willr_start returned a negative index");
                        let output_len = high.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_willr(
                            high.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_willr returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("willr", "talib", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_willr_start(options[0]);
            assert!(start_index >= 0, "ta_willr_start returned a negative index");
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("willr_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("TA-Lib WILLR {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_willr(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_willr returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_willr_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let high: Vec<f64> = data.iter().map(|d| d.high).collect();
                let low: Vec<f64> = data.iter().map(|d| d.low).collect();
                let close: Vec<f64> = data.iter().map(|d| d.close).collect();
                (symbol.clone(), high, low, close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3], // high, low, close
            &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3], // high, low, close
            &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3], // high, low, close
            &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3], // high, low, close
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = tulip_rs::indicators::willr::indicator_by_assets::<4>(
                        &inputs, &options, None,
                    )
                    .expect("Rust SIMD by assets WILLR indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "willr",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low, close) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high, &low, &close],
            &[&high, &low, &close],
            &[&high, &low, &close],
            &[&high, &low, &close],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("willr_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("SIMD by assets WILLR {{ {:.1} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = tulip_rs::indicators::willr::indicator_by_assets::<4>(
                            &inputs, &options, None,
                        )
                        .expect("Rust SIMD by assets WILLR indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_willr_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("willr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Create options array with 4 option sets for 4-lane SIMD processing
            let options_4 = [
                &OPTIONS_LIST[0], // [25.0]
                &OPTIONS_LIST[1], // [35.0]
                &OPTIONS_LIST[2], // [50.0]
                &OPTIONS_LIST[3], // [100.0]
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = tulip_rs::indicators::willr::indicator_by_options::<4>(
                        &inputs, &options_4, None,
                    )
                    .expect("Rust SIMD by options WILLR indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "willr",
                "Rust_SIMD",
                &[0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        // Create options array with 4 option sets for 4-lane SIMD processing
        let options_4 = [
            &OPTIONS_LIST[0], // [25.0]
            &OPTIONS_LIST[1], // [35.0]
            &OPTIONS_LIST[2], // [50.0]
            &OPTIONS_LIST[3], // [100.0]
        ];

        // Benchmark all 4 options together with 4-lane SIMD
        let mut group = c.benchmark_group("willr_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("SIMD by options WILLR (4 lanes)", |b| {
            b.iter(|| {
                let result = tulip_rs::indicators::willr::indicator_by_options::<4>(
                    &inputs, &options_4, None,
                )
                .expect("Rust SIMD by options WILLR indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_willr_simd_by_options,
    bench_rust_willr_simd_by_assets,
    bench_rust_willr,
    bench_c_willr,
    bench_talib_willr,
    bench_rust_willr_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_willr_simd_by_options,
    bench_rust_willr_simd_by_assets,
    bench_rust_willr,
    bench_c_willr,
    bench_rust_willr_from_state,
);
criterion_main!(benches);
