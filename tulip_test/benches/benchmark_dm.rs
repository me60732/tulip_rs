use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::dm::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_dm, ti_dm_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Test data from dm_test.rs
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];

const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

// Options from dm_test.rs
const OPTIONS_LIST: [[f64; 1]; 4] = [[24.0], [14.0], [5.0], [30.0]];

// Chunk size for from_state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for synthetic benchmarking
fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..499 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

/// Extract HL arrays from stock data
fn get_hl_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    (high, low)
}

fn bench_c_dm(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("dm");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = unsafe { ti_dm_start(options.as_ptr()) };
                        let output_len = high.len() - (start_index as usize);
                        let mut plus_dm_vec = vec![0.0_f64; output_len];
                        let mut minus_dm_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> =
                            vec![plus_dm_vec.as_mut_ptr(), minus_dm_vec.as_mut_ptr()];
                        let ret = unsafe {
                            ti_dm(
                                high.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_dm returned error code {}", ret);
                        black_box(&plus_dm_vec);
                        black_box(&minus_dm_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("dm", "C_tulip", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("C DM {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
                    let start_index = unsafe { ti_dm_start(options.as_ptr()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut plus_dm_vec = vec![0.0_f64; output_len];
                    let mut minus_dm_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> =
                        vec![plus_dm_vec.as_mut_ptr(), minus_dm_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_dm(
                            high.len() as i32,
                            black_box(&inputs).as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_dm returned error code {}", ret);
                    black_box(&plus_dm_vec);
                    black_box(&minus_dm_vec);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_dm(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("dm");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None); //.expect("DM indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("dm", "Rust", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("Rust DM {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs = [high.as_slice(), low.as_slice()];
                    let result = indicator(&inputs, &options, None).expect("DM indicator failed");
                    black_box(&result);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_dm_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("dm");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&high[..min_data_val], &low[..min_data_val]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("DM indicator failed");

                        // Chunks
                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref())
                        {
                            let chunk_inputs = [high_chunk, low_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();

                        if !high_rem.is_empty() {
                            let chunk_inputs = [high_rem, low_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "dm",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [&high[..high.len() - 1], &low[..low.len() - 1]];
                    let final_inputs = [&high[high.len() - 1..], &low[low.len() - 1..]];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust DM indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust DM from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "dm",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust DM indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust DM from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "dm",
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
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(&format!("Rust DM from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&high[..min_data_val], &low[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("DM indicator failed");

                    // Chunks
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let chunk_inputs = [high_chunk, low_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });

            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of DM.
fn bench_rust_dm_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("dm");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low) = get_hl_arrays(data);
                (symbol.clone(), high, low)
            })
            .collect();

        // Test each option set
        for options in &OPTIONS_LIST {
            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 2]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                ],
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, options, None)
                        .expect("Rust SIMD by assets DM indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "dm",
                "Rust_SIMD_by_assets",
                options,
                stock_data[0].1.len(),
                &timing,
                Some(&format!("4_Assets_Period_{}", options[0])),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();

        // Test each option set
        for options in &OPTIONS_LIST {
            // Create 4 identical datasets for SIMD processing
            let inputs: [&[&[f64]; 2]; 4] = [
                &[&high_vec, &low_vec],
                &[&high_vec, &low_vec],
                &[&high_vec, &low_vec],
                &[&high_vec, &low_vec],
            ];

            let mut group = c.benchmark_group("dm_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets DM period {}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, options, None)
                            .expect("Rust SIMD by assets DM indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_dm_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    if should_log_to_db() {
        init_database_data();
        init_logging("dm");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high_vec, low_vec) = get_hl_arrays(&stock_data);
            let inputs = [high_vec.as_slice(), low_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD DM indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "dm",
                "Rust_SIMD",
                &[0.0],
                high_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        let mut group = c.benchmark_group("dm_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options DM (3 lanes)", |b| {
            b.iter(|| {
                // Process all 3 options with 3-wide SIMD
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD DM indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    dm_benchmarks,
    bench_rust_dm_simd_by_options,
    bench_rust_dm_simd_by_assets,
    bench_rust_dm,
    bench_c_dm,
    bench_rust_dm_from_state,
);
criterion_main!(dm_benchmarks);
