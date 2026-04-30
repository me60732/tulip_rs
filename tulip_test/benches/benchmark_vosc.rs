use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::vosc::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_vosc, ti_vosc_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data from vosc_test.rs
const VOLUME: [f64; 15] = [
    5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
    4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
];

// Options for VOSC (fast_period, slow_period)
const OPTIONS_LIST: [[f64; 2]; 4] = [[2.0, 5.0], [5.0, 20.0], [10.0, 25.0], [14.0, 28.0]];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> Vec<f64> {
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..500 {
        volume_vec.extend_from_slice(&VOLUME);
    }
    volume_vec
}

/// Benchmark the C implementation of VOSC.
fn bench_c_vosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs: Vec<*const f64> = vec![volume_vec.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_vosc_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_vosc_start returned a negative index");
                        let output_len = volume_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_vosc(
                                volume_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_vosc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vosc",
                    "C_tulip",
                    &options,
                    volume_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let volume_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![volume_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_vosc_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vosc_start returned a negative index");
            let output_len = volume_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("vosc_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("C VOSC {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_vosc(
                                volume_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_vosc returned error code {}", ret);
                        black_box(&output_vec);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of VOSC.
fn bench_rust_vosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [volume_vec.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("Rust VOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vosc",
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
        let volume_vec = expand_inputs();
        let inputs = [volume_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("vosc_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust VOSC {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result =
                            indicator(&inputs, &options, None).expect("Rust VOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of VOSC.
fn bench_rust_vosc_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let n = volume_vec.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&volume_vec[..min_data_val]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("VOSC indicator failed");

                        // Chunks
                        let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for volume_chunk in volume_chunks.by_ref() {
                            let result = state.batch_indicator(&[volume_chunk], None);
                            black_box(&result);
                        }

                        // Remainder
                        let volume_rem = volume_chunks.remainder();

                        if !volume_rem.is_empty() {
                            let result = state.batch_indicator(&[volume_rem], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vosc",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if volume_vec.len() > 1 {
                    let new_inputs = [&volume_vec[..volume_vec.len() - 1]];
                    let final_inputs = [&volume_vec[volume_vec.len() - 1..]];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust VOSC indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust VOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "vosc",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust VOSC indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust VOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "vosc",
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
        let volume_vec = expand_inputs();
        let _inputs = [&volume_vec];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(&format!(
                "Rust VOSC from state {{ {}, {} }}",
                options[0], options[1]
            ));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&volume_vec[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("VOSC indicator failed");

                    // Chunks
                    let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for volume_chunk in volume_chunks.by_ref() {
                        let chunk_inputs = [volume_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let volume_rem = volume_chunks.remainder();

                    if !volume_rem.is_empty() {
                        let chunk_inputs = [volume_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if volume_vec.len() > 1 {
                let new_inputs = [&volume_vec[..volume_vec.len() - 1]];
                let final_inputs = [&volume_vec[volume_vec.len() - 1..]];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust VOSC indicator failed");

                let mut group = c.benchmark_group(&format!(
                    "Rust VOSC from state 1 bar {{ {}, {} }}",
                    options[0], options[1]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust VOSC from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of VOSC.
fn bench_rust_vosc_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4 for SIMD processing
        let stock_data: Vec<_> = data.into_iter().collect();
        let chunks: Vec<_> = stock_data.chunks(4).collect();

        for chunk in chunks {
            let stock_symbols: Vec<_> = chunk.iter().map(|(symbol, _)| symbol.as_str()).collect();
            let volume_arrays: Vec<_> = chunk
                .iter()
                .map(|(_, data)| data.iter().map(|d| d.volume).collect::<Vec<_>>())
                .collect();

            // Pad to 4 assets if needed
            let mut padded_volume = volume_arrays.clone();
            let mut padded_symbols = stock_symbols.clone();
            while padded_volume.len() < 4 {
                padded_volume.push(padded_volume[0].clone());
                padded_symbols.push("PADDING");
            }

            for options in OPTIONS_LIST {
                let min_len = padded_volume.iter().map(|v| v.len()).min().unwrap_or(0);
                if min_len < min_data(&options) {
                    continue;
                }

                // Prepare inputs for SIMD
                let inputs: [&[&[f64]; 1]; 4] = [
                    &[padded_volume[0].as_slice()],
                    &[padded_volume[1].as_slice()],
                    &[padded_volume[2].as_slice()],
                    &[padded_volume[3].as_slice()],
                ];

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        use tulip_rs::indicators::vosc::indicator_by_assets;
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD VOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vosc",
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
        let volume_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let inputs: [&[&[f64]; 1]; 4] = [
                &[volume_vec.as_slice()],
                &[volume_vec.as_slice()],
                &[volume_vec.as_slice()],
                &[volume_vec.as_slice()],
            ];

            let mut group = c.benchmark_group("vosc_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("SIMD VOSC by assets {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        use tulip_rs::indicators::vosc::indicator_by_assets;
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD VOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of VOSC with optional outputs.
fn bench_rust_vosc_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [volume_vec.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust VOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vosc",
                    "Rust_optional",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let volume_vec = expand_inputs();
        let inputs = [volume_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("vosc_rust_optional");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust VOSC optional {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust VOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_vosc_simd_by_options(c: &mut Criterion) {
    use tulip_rs::indicators::vosc::indicator_by_options;

    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];

    if should_log_to_db() {
        init_database_data();
        init_logging("vosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [volume_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process all 4 options with 4-wide SIMD
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD by options VOSC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "vosc",
                "Rust_SIMD",
                &[0.0],
                volume_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let volume_vec = expand_inputs();
        let inputs = [volume_vec.as_slice()];

        let mut group = c.benchmark_group("vosc_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options VOSC (4 lanes)", |b| {
            b.iter(|| {
                let result = tulip_rs::indicators::vosc::indicator_by_options::<4>(
                    &inputs, &options_4, None,
                )
                .expect("Rust SIMD by options VOSC indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_vosc_simd_by_options,
    bench_rust_vosc_simd_by_assets,
    bench_rust_vosc,
    bench_c_vosc,
    bench_rust_vosc_from_state,
    bench_rust_vosc_optional,
);
criterion_main!(benches);
