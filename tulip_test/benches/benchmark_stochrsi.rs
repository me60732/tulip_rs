use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::stochrsi::{
    indicator as rust_stochrsi, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_stochrsi, ti_stochrsi_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
const SAMPLE_SIZE: usize = 30000;
// Test data from stochrsi_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options from stochrsi_test.rs
/* const OPTIONS_LIST: [[f64; 1]; 8] = [[5.0], [7.0], [8.0], [10.0], [14.0], [20.0], [25.0], [35.0]]; */
const OPTIONS_LIST: [[f64; 1]; 4] = [[14.0], [20.0], [25.0], [35.0]];
/*const OPTIONS_LIST: [[f64; 1]; 10] = [
    [5.0],
    [7.0],
    [8.0],
    [10.0],
    [14.0],
    [20.0],
    [25.0],
    [35.0],
    [50.0],
    [100.0],
];*/
/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for synthetic benchmarking
fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..499 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

/// Extract close price array from stock data
fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

fn bench_c_stochrsi(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_stochrsi_start(options.as_ptr()) };
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_stochrsi(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_stochrsi returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "stochrsi",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let close = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("C STOCHRSI {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs: Vec<*const f64> = vec![black_box(&close).as_ptr()];
                    let start_index = unsafe { ti_stochrsi_start(options.as_ptr()) };
                    let output_len = close.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_stochrsi(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_stochrsi returned error code {}", ret);
                    black_box(&output_vec);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_stochrsi(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = rust_stochrsi(&inputs, &options, None)
                            .expect("STOCHRSI indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "stochrsi",
                    "Rust",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let close = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!("Rust STOCHRSI {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs = [close.as_slice()];
                    let result =
                        rust_stochrsi(&inputs, &options, None).expect("STOCHRSI indicator failed");
                    black_box(&result);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_stochrsi_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) = rust_stochrsi(&chunk_inputs, &options, None)
                            .expect("STOCHRSI indicator failed");

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
                    "stochrsi",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if close.len() > 1 {
                    let new_inputs = [&close[..close.len() - 1]];
                    let final_inputs = [&close[close.len() - 1..]];
                    let (_, mut state) = rust_stochrsi(&new_inputs, &options, None)
                        .expect("Rust STOCHRSI indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust STOCHRSI from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "stochrsi",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = rust_stochrsi(&new_inputs, &options, None)
                        .expect("Rust STOCHRSI indicator failed");

                    let json = serde_json::to_string(&state).expect("json failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust STOCHRSI from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "stochrsi",
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
        let close = expand_inputs();
        let _inputs = [&close];

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(&format!("Rust STOCHRSI from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close[..min_data]];

                    let (_, mut state) = rust_stochrsi(&chunk_inputs, &options, None)
                        .expect("STOCHRSI indicator failed");

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
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if close.len() > 1 {
                let new_inputs = [&close[..close.len() - 1]];
                let final_inputs = [&close[close.len() - 1..]];
                let (_, mut state) = rust_stochrsi(&new_inputs, &options, None)
                    .expect("Rust STOCHRSI indicator failed");

                let mut group = c.benchmark_group(&format!(
                    "Rust STOCHRSI from state 1 bar {{ {:.1} }}",
                    options[0]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust STOCHRSI from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust implementation of STOCHRSI with optional outputs.
fn bench_rust_stochrsi_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = rust_stochrsi(&inputs, &options, Some(&[true]))
                            .expect("Rust STOCHRSI indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "stochrsi",
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
        let close = expand_inputs();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("stochrsi_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust STOCHRSI {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = rust_stochrsi(&inputs, &options, Some(&[true]))
                        .expect("Rust STOCHRSI indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_stochrsi_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
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
                    let result = tulip_rs::indicators::stochrsi::indicator_by_assets::<4>(
                        &inputs, &options, None,
                    )
                    .expect("Rust SIMD by assets STOCHRSI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "stochrsi",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("all"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 1]; 4] = [&[&close], &[&close], &[&close], &[&close]];

        for options in OPTIONS_LIST {
            c.bench_function(
                &format!("SIMD by assets STOCHRSI {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = tulip_rs::indicators::stochrsi::indicator_by_assets::<4>(
                            &inputs, &options, None,
                        )
                        .expect("Rust SIMD by assets STOCHRSI indicator failed");
                        black_box(&result);
                    });
                },
            );
        }
    }
}

fn bench_rust_stochrsi_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    if should_log_to_db() {
        init_database_data();
        init_logging("stochrsi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD STOCHRSI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "stochrsi",
                "Rust_SIMD",
                &[0.0],
                close.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close = expand_inputs();
        let inputs = [close.as_slice()];

        let mut group = c.benchmark_group("stochrsi_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("Rust SIMD by options STOCHRSI (4 lanes)", |b| {
            b.iter(|| {
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD STOCHRSI indicator failed");
                black_box(&result);
            });
        });

        group.finish();
    }
}

criterion_group!(
    stochrsi_benchmarks,
    bench_rust_stochrsi,
    bench_c_stochrsi,
    bench_rust_stochrsi_simd_by_assets,
    bench_rust_stochrsi_simd_by_options,
    bench_rust_stochrsi_from_state,
    bench_rust_stochrsi_optional,
);
criterion_main!(stochrsi_benchmarks);
