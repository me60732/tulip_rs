use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::vwma::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_vwma, ti_vwma_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data from vwma_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];
const VOLUME: [f64; 15] = [
    5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
    4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
];

// Options for VWMA (period)
const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0]];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut close_vec = CLOSE.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..500 {
        close_vec.extend_from_slice(&CLOSE);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (close_vec, volume_vec)
}

/// Benchmark the C implementation of VWMA.
fn bench_c_vwma(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs: Vec<*const f64> = vec![close_vec.as_ptr(), volume_vec.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_vwma_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_vwma_start returned a negative index");
                        let output_len = close_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_vwma(
                                close_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_vwma returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwma",
                    "C_tulip",
                    &options,
                    close_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr(), volume_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_vwma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vwma_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("vwma_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("C VWMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_vwma(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_vwma returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of VWMA.
fn bench_rust_vwma(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("Rust VWMA indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwma",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("vwma_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust VWMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust VWMA indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of VWMA.
fn bench_rust_vwma_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let n = close_vec.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs =
                            [&close_vec[..min_data_val], &volume_vec[..min_data_val]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("VWMA indicator failed");

                        // Chunks
                        let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (close_chunk, volume_chunk) in
                            close_chunks.by_ref().zip(volume_chunks.by_ref())
                        {
                            let result = state.batch_indicator(&[close_chunk, volume_chunk], None);
                            black_box(&result);
                        }

                        // Remainder
                        let close_rem = close_chunks.remainder();
                        let volume_rem = volume_chunks.remainder();

                        if !close_rem.is_empty() && !volume_rem.is_empty() {
                            let close_rem_vec = close_rem.to_vec();
                            let volume_rem_vec = volume_rem.to_vec();
                            let result =
                                state.batch_indicator(&[&close_rem_vec, &volume_rem_vec], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwma",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if close_vec.len() > 1 {
                    let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                    let new_volume_vec = volume_vec[..volume_vec.len() - 1].to_vec();
                    let new_inputs = [new_close_vec.as_slice(), new_volume_vec.as_slice()];
                    let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                    let final_volume_vec = volume_vec[volume_vec.len() - 1..].to_vec();
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust VWMA indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(
                                    &[final_close_vec.as_slice(), final_volume_vec.as_slice()],
                                    None,
                                )
                                .expect("Rust VWMA from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "vwma",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust VWMA indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(
                                    &[final_close_vec.as_slice(), final_volume_vec.as_slice()],
                                    None,
                                )
                                .expect("Rust VWMA from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "vwma",
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
        let (close_vec, volume_vec) = expand_inputs();
        let _inputs = [&close_vec, &volume_vec];

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(&format!("Rust VWMA from state {{ {} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data_val], &volume_vec[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("VWMA indicator failed");

                    // Chunks
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (close_chunk, volume_chunk) in
                        close_chunks.by_ref().zip(volume_chunks.by_ref())
                    {
                        let result = state.batch_indicator(&[close_chunk, volume_chunk], None);
                        black_box(&result);
                    }

                    // Remainder
                    let close_rem = close_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !close_rem.is_empty() && !volume_rem.is_empty() {
                        let close_rem_vec = close_rem.to_vec();
                        let volume_rem_vec = volume_rem.to_vec();
                        let result =
                            state.batch_indicator(&[&close_rem_vec, &volume_rem_vec], None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                let new_volume_vec = volume_vec[..volume_vec.len() - 1].to_vec();
                let new_inputs = [new_close_vec.as_slice(), new_volume_vec.as_slice()];

                let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                let final_volume_vec = volume_vec[volume_vec.len() - 1..].to_vec();
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust VWMA indicator failed");

                let mut group =
                    c.benchmark_group(&format!("Rust VWMA from state 1 bar {{ {} }}", options[0]));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(
                                &[final_close_vec.as_slice(), final_volume_vec.as_slice()],
                                None,
                            )
                            .expect("Rust VWMA from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of VWMA.
fn bench_rust_vwma_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwma");

        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4 for SIMD processing
        let stock_data: Vec<_> = data.into_iter().collect();
        let chunks: Vec<_> = stock_data.chunks(4).collect();

        for chunk in chunks {
            let stock_symbols: Vec<_> = chunk.iter().map(|(symbol, _)| symbol.as_str()).collect();
            let close_volume_arrays: Vec<_> = chunk
                .iter()
                .map(|(_, data)| {
                    let close: Vec<f64> = data.iter().map(|d| d.close).collect();
                    let volume: Vec<f64> = data.iter().map(|d| d.volume).collect();
                    (close, volume)
                })
                .collect();

            // Pad to 4 assets if needed
            let mut padded_cv = close_volume_arrays.clone();
            let mut padded_symbols = stock_symbols.clone();
            while padded_cv.len() < 4 {
                padded_cv.push(padded_cv[0].clone());
                padded_symbols.push("PADDING");
            }

            for options in OPTIONS_LIST {
                let min_len = padded_cv.iter().map(|(c, _)| c.len()).min().unwrap_or(0);
                if min_len < min_data(&options) {
                    continue;
                }

                // Prepare inputs for SIMD
                let inputs: [&[&[f64]; 2]; 4] = [
                    &[padded_cv[0].0.as_slice(), padded_cv[0].1.as_slice()],
                    &[padded_cv[1].0.as_slice(), padded_cv[1].1.as_slice()],
                    &[padded_cv[2].0.as_slice(), padded_cv[2].1.as_slice()],
                    &[padded_cv[3].0.as_slice(), padded_cv[3].1.as_slice()],
                ];

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        use tulip_rs::indicators::vwma::indicator_by_assets;
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD VWMA indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwma",
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
        let (close_vec, volume_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let inputs: [&[&[f64]; 2]; 4] = [
                &[close_vec.as_slice(), volume_vec.as_slice()],
                &[close_vec.as_slice(), volume_vec.as_slice()],
                &[close_vec.as_slice(), volume_vec.as_slice()],
                &[close_vec.as_slice(), volume_vec.as_slice()],
            ];

            let mut group = c.benchmark_group("vwma_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("SIMD VWMA by assets {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    use tulip_rs::indicators::vwma::indicator_by_assets;
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("SIMD VWMA indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

// SIMD-by-options benchmark for VWMA (4+2 lanes)
fn bench_rust_vwma_simd_by_options(c: &mut Criterion) {
    use tulip_rs::indicators::vwma::indicator_by_options;

    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];

    if should_log_to_db() {
        init_database_data();
        init_logging("vwma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process first 4 options with 4-wide SIMD
                    let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD VWMA indicator failed");
                    black_box(&result_4);

                    // Process remaining 2 options with 2-wide SIMD
                    let result_2 = indicator_by_options::<2>(&inputs, &options_2, None)
                        .expect("Rust SIMD VWMA indicator failed");
                    black_box(&result_2);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "vwma",
                "Rust_SIMD",
                &[0.0],
                close_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

        let mut group = c.benchmark_group("vwma_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD VWMA (4+2 lanes)", |b| {
            b.iter(|| {
                // Process first 4 options with 4-wide SIMD
                let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD VWMA indicator failed");
                black_box(&result_4);

                // Process remaining 2 options with 2-wide SIMD
                let result_2 = indicator_by_options::<2>(&inputs, &options_2, None)
                    .expect("Rust SIMD VWMA indicator failed");
                black_box(&result_2);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_vwma_simd_by_options,
    bench_rust_vwma_simd_by_assets,
    bench_rust_vwma,
    bench_c_vwma,
    bench_rust_vwma_from_state,
);
criterion_main!(benches);
