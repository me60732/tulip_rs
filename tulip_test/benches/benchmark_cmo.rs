use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::cmo::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_cmo, ti_cmo_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data from cmo_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for CMO (period)
const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

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

/// Benchmark the C implementation of CMO.
fn bench_c_cmo(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("cmo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_cmo_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_cmo_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_cmo(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_cmo returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("cmo", "C_tulip", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_cmo_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_cmo_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("cmo_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C CMO {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_cmo(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_cmo returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of CMO.
fn bench_rust_cmo(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("cmo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("Rust CMO indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("cmo", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("cmo_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust CMO {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust CMO indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of CMO.
fn bench_rust_cmo_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("cmo");

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

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("CMO indicator failed");

                        // Chunks
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

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
                    "cmo",
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
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust CMO indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust CMO from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "cmo",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust CMO indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust CMO from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "cmo",
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
            let mut group = c.benchmark_group(format!("Rust CMO from state {{ {} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("CMO indicator failed");

                    // Chunks
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

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
                    indicator(&new_inputs, &options, None).expect("Rust CMO indicator failed");

                let mut group =
                    c.benchmark_group(format!("Rust CMO from state 1 bar {{ {} }}", options[0]));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust CMO from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the SIMD-by-assets CMO implementation.
fn bench_rust_cmo_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("cmo");

        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4 for SIMD processing
        let stock_data: Vec<_> = data.iter().collect();
        let chunks: Vec<_> = stock_data.chunks(4).collect();

        for chunk in chunks {
            let stock_symbols: Vec<_> = chunk.iter().map(|(symbol, _)| symbol.as_str()).collect();
            let close_arrays: Vec<_> = chunk
                .iter()
                .map(|(_, data)| get_close_array(data))
                .collect();

            // Pad to 4 assets if needed
            let mut padded_close = close_arrays.clone();
            let mut padded_symbols = stock_symbols.clone();
            while padded_close.len() < 4 {
                padded_close.push(padded_close[0].clone());
                padded_symbols.push("PADDING");
            }

            for options in OPTIONS_LIST {
                let min_len = padded_close.iter().map(|c| c.len()).min().unwrap_or(0);
                if min_len < min_data(&options) {
                    continue;
                }

                // Prepare inputs for SIMD
                let inputs: [&[&[f64]; 1]; 4] = [
                    &[padded_close[0].as_slice()],
                    &[padded_close[1].as_slice()],
                    &[padded_close[2].as_slice()],
                    &[padded_close[3].as_slice()],
                ];

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD CMO indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "cmo",
                    "Rust_SIMD_by_assets",
                    &options,
                    min_len,
                    &timing,
                    Some("All"),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let inputs: [&[&[f64]; 1]; 4] = [
                &[close_vec.as_slice()],
                &[close_vec.as_slice()],
                &[close_vec.as_slice()],
                &[close_vec.as_slice()],
            ];

            let mut group = c.benchmark_group("cmo_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("SIMD CMO by assets {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("SIMD CMO indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by options implementation of CMO.
fn bench_rust_cmo_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("cmo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [close_vec.as_slice()];

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
                        .expect("Rust SIMD CMO indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "cmo",
                "Rust_SIMD",
                &[0.0],
                close_vec.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("cmo_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options CMO (4 lanes)", |b| {
            b.iter(|| {
                // Process all 4 options with 4-wide SIMD
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD CMO indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_cmo_simd_by_options,
    bench_rust_cmo_simd_by_assets,
    bench_rust_cmo,
    bench_c_cmo,
    bench_rust_cmo_from_state,
);
criterion_main!(benches);
