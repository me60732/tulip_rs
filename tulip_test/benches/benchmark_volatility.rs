use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::volatility::indicator_by_options;
use tulip_rs::indicators::volatility::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_volatility, ti_volatility_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data (close prices)
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for VOLATILITY (period)
//const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0]];
const OPTIONS_LIST: [[f64; 1]; 4] = [[14.0], [20.0], [25.0], [30.0]];
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

/// Benchmark the C implementation of VOLATILITY.
fn bench_c_volatility(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("volatility");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_volatility_start(options.as_ptr()) };
                        assert!(
                            start_index >= 0,
                            "ti_volatility_start returned a negative index"
                        );
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_volatility(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_volatility returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "volatility",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_volatility_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_volatility_start returned a negative index"
            );
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("volatility_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C VOLATILITY {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_volatility(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_volatility returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of VOLATILITY.
fn bench_rust_volatility(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("volatility");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust VOLATILITY indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "volatility",
                    "Rust",
                    &options,
                    n,
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
            let mut group = c.benchmark_group("volatility_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust VOLATILITY {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &options, None)
                        .expect("Rust VOLATILITY indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_volatility_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("volatility");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close[..min_data_val]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("VOLATILITY indicator failed");

                        // Chunks
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

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
                    "volatility",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if close.len() > 1 {
                    let new_inputs = [&close[..close.len() - 1]];
                    let final_inputs = [&close[close.len() - 1..]];
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust VOLATILITY indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust VOLATILITY from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "volatility",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust VOLATILITY indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust VOLATILITY from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "volatility",
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
            let mut group =
                c.benchmark_group(format!("Rust VOLATILITY from state {{ {} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data_val]];

                    let (_, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("VOLATILITY indicator failed");

                    // Chunks
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

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
                let new_inputs = [&close_vec[..close_vec.len() - 1]];
                let final_inputs = [&close_vec[close_vec.len() - 1..]];
                let (_, mut state) = indicator(&new_inputs, &options, None)
                    .expect("Rust VOLATILITY indicator failed");

                let mut group = c.benchmark_group(format!(
                    "Rust VOLATILITY from state 1 bar {{ {} }}",
                    options[0]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust VOLATILITY from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

fn bench_rust_volatility_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("volatility");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let close = get_close_array(data);
                (symbol.clone(), close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1], // close
            &[&stock_data[1].1], // close
            &[&stock_data[2].1], // close
            &[&stock_data[3].1], // close
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = tulip_rs::indicators::volatility::indicator_by_assets::<4>(
                        &inputs, &options, None,
                    )
                    .expect("Rust SIMD by assets VOLATILITY indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "volatility",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 1]; 4] =
            [&[&close_vec], &[&close_vec], &[&close_vec], &[&close_vec]];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("volatility_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust SIMD by assets VOLATILITY {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = tulip_rs::indicators::volatility::indicator_by_assets::<4>(
                            &inputs, &options, None,
                        )
                        .expect("Rust SIMD by assets VOLATILITY indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by options implementation of VOLATILITY.
fn bench_rust_volatility_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    if should_log_to_db() {
        init_database_data();
        init_logging("volatility");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process 4 options with 4-wide SIMD
                    let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD VOLATILITY indicator failed");
                    black_box(&result_4);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "volatility",
                "Rust_SIMD",
                &[0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("volatility_rust_simd");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD VOLATILITY (4 lanes)", |b| {
            b.iter(|| {
                // Process 4 options with 4-wide SIMD
                let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD VOLATILITY indicator failed");
                black_box(&result_4);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_volatility_simd_by_assets,
    bench_rust_volatility_simd_by_options,
    bench_rust_volatility,
    bench_c_volatility,
    bench_rust_volatility_from_state,
);
criterion_main!(benches);
