use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::ppo::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_ppo, ti_ppo_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ppo, ta_ppo_start};

// Test input data (close prices) - copied from test file
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for PPO - copied from test file
const OPTIONS_LIST: [[f64; 2]; 4] = [[2.0, 5.0], [12.0, 26.0], [9.0, 20.0], [8.0, 18.0]];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for profiling
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

fn bench_c_ppo(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];
            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_ppo_start(options.as_ptr()) };
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_ppo(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_ppo returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ppo", "C_tulip", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_ppo_start(options.as_ptr()) };
            let output_len = close_vec.len() - (start_index as usize);

            let mut group =
                c.benchmark_group(format!("C PPO {{ {:.1}, {:.1} }}", options[0], options[1]));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("C PPO {{ {:.1}, {:.1} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_ppo(
                                close_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_ppo returned error code {}", ret);
                        black_box(&output_vec);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ppo(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

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
                            indicator(&inputs, &options, None).expect("PPO indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ppo", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!(
                "Rust PPO {{ {:.1}, {:.1} }}",
                options[0], options[1]
            ));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust PPO {{ {:.1}, {:.1} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result =
                            indicator(&inputs, &options, None).expect("PPO indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ppo_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("PPO indicator failed");

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
                    "ppo",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if close.len() > 1 {
                    let new_close_vec = close[..close.len() - 1].to_vec();
                    let new_inputs = [new_close_vec.as_slice()];
                    let final_close_vec = close[close.len() - 1..].to_vec();
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust PPO indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&[final_close_vec.as_slice()], None)
                                .expect("Rust PPO from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ppo",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust PPO indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&[final_close_vec.as_slice()], None)
                                .expect("Rust PPO from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ppo",
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
            let mut group = c.benchmark_group(format!(
                "Rust PPO from state {{ {:.1}, {:.1} }}",
                options[0], options[1]
            ));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("PPO indicator failed");

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
                let new_close_vec = close_vec[..close_vec.len() - 1].to_vec();
                let new_inputs = [new_close_vec.as_slice()];
                let final_close_vec = close_vec[close_vec.len() - 1..].to_vec();
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust PPO indicator failed");

                let mut group = c.benchmark_group(format!(
                    "Rust PPO from state 1 bar {{ {:.1}, {:.1} }}",
                    options[0], options[1]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&[final_close_vec.as_slice()], None)
                            .expect("Rust PPO from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the TA-Lib implementation of PPO.
#[cfg(feature = "talib")]
fn bench_talib_ppo(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_ppo_start(options[0], options[1]);
                        assert!(start_index >= 0, "ta_ppo_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_ppo(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_ppo returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("ppo", "talib", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_ppo_start(options[0], options[1]);
            assert!(start_index >= 0, "ta_ppo_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("ppo_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("TA-Lib PPO {{ {:.1}, {:.1} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = ta_ppo(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_ppo returned error code {}", ret);
                        black_box(&output_vec);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of PPO with optional outputs.
fn bench_rust_ppo_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust PPO indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ppo",
                    "Rust_optional",
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
            let mut group = c.benchmark_group("ppo_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust PPO {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust PPO indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ppo_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
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
                        .expect("Rust SIMD by assets PPO indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ppo",
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
            let mut group = c.benchmark_group("ppo_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust SIMD by assets PPO {{ {}, {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets PPO indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ppo_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec = get_close_array(stock_data);
            let n = close_vec.len();
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
                        .expect("Rust SIMD PPO indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ppo",
                "Rust_SIMD",
                &[0.0, 0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("ppo_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options PPO (4 lanes)", |b| {
            b.iter(|| {
                // Process all 4 options with 4-wide SIMD
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result =
                    tulip_rs::indicators::ppo::indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD PPO indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the `ta` crate (RustTa) implementation of PPO.
fn bench_rust_ta_ppo(c: &mut Criterion) {
    use ta::indicators::PercentagePriceOscillator as Ppo;
    use ta::Next;

    if should_log_to_db() {
        init_database_data();
        init_logging("ppo");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec = get_close_array(stock_data);
            let n = close_vec.len();

            for options in OPTIONS_LIST {
                let fast = options[0] as usize;
                let slow = options[1] as usize;
                let signal = 9_usize;
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut ppo = Ppo::new(fast, slow, signal).expect("ta PPO new failed");
                        let mut last = 0.0_f64;
                        for &price in &close_vec {
                            let out = ppo.next(price);
                            last = out.ppo;
                        }
                        black_box(last);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("ppo", "RustTa", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let fast = options[0] as usize;
            let slow = options[1] as usize;
            let signal = 9_usize;
            let mut group = c.benchmark_group("ppo_rust_ta");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("RustTa PPO {{ {}/{} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut ppo = Ppo::new(fast, slow, signal).expect("ta PPO new failed");
                        let mut last = 0.0_f64;
                        for &price in &close_vec {
                            let out = ppo.next(price);
                            last = out.ppo;
                        }
                        black_box(last);
                    });
                },
            );
            group.finish();
        }
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    ppo_benchmarks,
    bench_rust_ppo_simd_by_options,
    bench_rust_ppo_simd_by_assets,
    bench_rust_ppo,
    bench_c_ppo,
    bench_talib_ppo,
    bench_rust_ppo_from_state,
    bench_rust_ppo_optional,
    bench_rust_ta_ppo,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    ppo_benchmarks,
    bench_rust_ppo_simd_by_options,
    bench_rust_ppo_simd_by_assets,
    bench_rust_ppo,
    bench_c_ppo,
    bench_rust_ppo_from_state,
    bench_rust_ppo_optional,
    bench_rust_ta_ppo,
);
criterion_main!(ppo_benchmarks);
