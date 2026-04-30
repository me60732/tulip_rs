use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::stddev::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_stddev, ti_stddev_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data (close prices)
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for STDDEV (period)
const OPTIONS_LIST: [[f64; 1]; 6] = [[10.0], [14.0], [20.0], [50.0], [100.0], [200.0]];
//const OPTIONS_LIST: [[f64; 1]; 4] = [[10.0], [14.0], [20.0], [50.0]];
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

/// Benchmark the C implementation of STDDEV.
fn bench_c_stddev(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stddev");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_stddev_start(options.as_ptr()) };
                        assert!(
                            start_index >= 0,
                            "ti_stddev_start returned a negative index"
                        );
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_stddev(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_stddev returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "stddev",
                    "C_tulip",
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
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_stddev_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_stddev_start returned a negative index"
            );
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("stddev_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("C STDDEV {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_stddev(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_stddev returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of STDDEV.
fn bench_rust_stddev(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stddev");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust STDDEV indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("stddev", "Rust", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("stddev_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust STDDEV {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust STDDEV indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of STDDEV.
fn bench_rust_stddev_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("stddev");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options);
                        // First chunk
                        let close_vec = close[..min_data].to_vec();
                        let chunk_inputs = [close_vec.as_slice()];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("STDDEV indicator failed");

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
                    "stddev",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_close_vec = close[..close.len() - 1].to_vec();
                    let new_inputs = [new_close_vec.as_slice()];
                    let final_close_vec = close[close.len() - 1..].to_vec();
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust STDDEV indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&[final_close_vec.as_slice()], None)
                                .expect("Rust STDDEV from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "stddev",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust STDDEV indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&[final_close_vec.as_slice()], None)
                                .expect("Rust STDDEV from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "stddev",
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

        for options in OPTIONS_LIST {
            let min_data = min_data(&options);
            // First chunk
            let close_chunk = close_vec[..min_data].to_vec();
            let chunk_inputs = [close_chunk.as_slice()];

            let (_, mut state) =
                indicator(&chunk_inputs, &options, None).expect("STDDEV indicator failed");

            let mut group = c.benchmark_group("stddev_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust STDDEV from state {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
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
                },
            );
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                let new_inputs = [new_close_vec.as_slice()];
                let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust STDDEV indicator failed");

                let mut group = c.benchmark_group("stddev_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    &format!("Rust STDDEV from state 1 bar {{ {} }}", options[0]),
                    |b| {
                        b.iter(|| {
                            let result = state
                                .batch_indicator(&[final_close_vec.as_slice()], None)
                                .expect("Rust STDDEV from state indicator failed");
                            black_box(&result);
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of STDDEV.
fn bench_rust_stddev_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stddev");

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
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets STDDEV indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "stddev",
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
            let mut group = c.benchmark_group("stddev_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets STDDEV {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets STDDEV indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of STDDEV with optional outputs.
fn bench_rust_stddev_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("stddev");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true]))
                            .expect("Rust STDDEV indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "stddev",
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
            let mut group = c.benchmark_group("stddev_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust STDDEV {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &options, Some(&[true]))
                        .expect("Rust STDDEV indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by options implementation of STDDEV (4 lanes + 2 lanes).
fn bench_rust_stddev_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];

    if should_log_to_db() {
        init_database_data();
        init_logging("stddev");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec = get_close_array(&stock_data);
            let inputs = [close_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process first 4 options with 4-wide SIMD

                    let _ = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD STDDEV 4-wide failed");

                    // Process remaining 2 options with 2-wide SIMD
                    let _ = indicator_by_options::<2>(&inputs, &options_2, None)
                        .expect("Rust SIMD STDDEV 2-wide failed");
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "stddev",
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

        let mut group = c.benchmark_group("stddev_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options STDDEV (4 lanes + 2 lanes)", |b| {
            b.iter(|| {
                // Process first 4 options with 4-wide SIMD
                let _ = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD STDDEV 4-wide failed");

                // Process remaining 2 options with 2-wide SIMD
                let _ = indicator_by_options::<2>(&inputs, &options_2, None)
                    .expect("Rust SIMD STDDEV 2-wide failed");

                black_box(());
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_stddev_simd_by_assets,
    bench_rust_stddev_simd_by_options,
    bench_rust_stddev,
    bench_c_stddev,
    bench_rust_stddev_from_state,
    bench_rust_stddev_optional,
);
criterion_main!(benches);
