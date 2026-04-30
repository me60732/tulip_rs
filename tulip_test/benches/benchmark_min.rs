use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::min::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_min, ti_min_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
use tulip_test::talib_bindings::{ta_min, ta_min_start};
//const SAMPLE_SIZE: usize = 1;
// Sample input data (close prices)
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for min (period)
//const OPTIONS_LIST: [[f64; 1]; 4] = [[25.0], [35.0], [50.0], [100.0]];
const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    //[7.0],
    [8.0],
    [10.0],
    [14.0],
    [25.0],
    [35.0],
    [50.0],
    [100.0],
    /*[150.0],
    [200.0],
    [300.0],*/
];

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

/// Benchmark the C implementation of min.
fn bench_c_min(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("min");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_min_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_min_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_min(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_min returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("min", "C_tulip", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_min_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_min_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("min_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("C min {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_min(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_min returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of min.
fn bench_rust_min(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("min");

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
                            indicator(&inputs, &options, None).expect("MIN indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("min", "Rust", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("min_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust min {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust min indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of min.
const CHUNK_SIZE: usize = 100;
fn bench_rust_min_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("min");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close[..min_data_val]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("MIN indicator failed");

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
                    "min",
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
                        indicator(&new_inputs, &options, None).expect("Rust min indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&[final_close.as_slice()], None)
                                .expect("Rust MIN from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "min",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust MIN indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&[final_close.as_slice()], None)
                                .expect("Rust MIN from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "min",
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
            let mut group =
                c.benchmark_group(&format!("Rust min from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let close_vec_chunk = close_vec[..min_data_val].to_vec();
                    let chunk_inputs = [close_vec_chunk.as_slice()];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("MIN indicator failed");

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
        }
    }
}

/// Benchmark the TA-Lib implementation of min.
fn bench_talib_min(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("min");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_min_start(options[0]);
                        assert!(start_index >= 0, "ta_min_start returned a negative index");
                        let output_len = close.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_min(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_min returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("min", "talib", &options, n, &timing, Some(&stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_min_start(options[0]);
            assert!(start_index >= 0, "ta_min_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("min_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("TA-Lib min {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_min(
                        close_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_min returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of min.
fn bench_rust_min_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("min");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            //.take(8)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
            /*&[&stock_data[4].1],
            &[&stock_data[5].1],
            &[&stock_data[6].1],
            &[&stock_data[7].1],*/
        ];
        /*let inputs_1 = [&[stock_data[0].1.as_slice()]];
        let inputs_2 = &[stock_data[1].1.as_slice()];
        let inputs_3 = &[stock_data[2].1.as_slice()];
        let inputs_4 = &[stock_data[3].1.as_slice()];*/
        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None);
                    //.expect("Rust SIMD by assets MIN indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "min",
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
            let mut group = c.benchmark_group("min_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets MIN {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets min indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

// Benchmark the Rust SIMD by options implementation of min.
fn bench_rust_min_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("min");

        let data = get_all_stock_data().unwrap();
        /*let options_8 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
            &OPTIONS_LIST[4],
            &OPTIONS_LIST[5],
            &OPTIONS_LIST[6],
            &OPTIONS_LIST[7],
        ];*/
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            let options_4_1 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let options_4_2 = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            //let options_1 = [&OPTIONS_LIST[8]];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process first 4 options with 4-wide SIMD

                    let result_4_1 = indicator_by_options::<4>(&inputs, &options_4_1, None)
                        .expect("Rust SIMD min indicator failed");
                    black_box(&result_4_1);

                    // Process next 4 options with 4-wide SIMD
                    let result_4_2 = indicator_by_options::<4>(&inputs, &options_4_2, None)
                        .expect("Rust SIMD min indicator failed");
                    black_box(&result_4_2);
                    /*let result_8 =
                        tulip_rs::indicators::nightly::by_option::min::indicator_by_option::<4>(
                            &inputs,
                            &options_8,
                            None,
                        )
                        .expect("Rust SIMD MIN indicator failed");
                    black_box(&result_8);*/
                    // Process remaining 1 option with scalar

                    /*let result_1 =
                        indicator(
                            &inputs, &OPTIONS_LIST[0], None,
                        )
                        .expect("Rust SIMD MIN indicator failed");
                    black_box(&result_1);*/
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "min",
                "Rust_SIMD",
                &[0.0],
                close.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("min_rust_simd");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD MIN (4+4+1 lanes)", |b| {
            b.iter(|| {
                // Process first 4 options with 4-wide SIMD
                let options_4_1 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result_4_1 = indicator_by_options::<4>(&inputs, &options_4_1, None)
                    .expect("Rust SIMD min indicator failed");
                black_box(&result_4_1);

                // Process next 4 options with 4-wide SIMD
                /*let options_4_2 = [
                    &OPTIONS_LIST[4],
                    &OPTIONS_LIST[5],
                    &OPTIONS_LIST[6],
                    &OPTIONS_LIST[7],
                ];
                let result_4_2 =
                    tulip_rs::indicators::nightly::by_option::min::indicator_by_option::<4>(
                        &inputs,
                        &options_4_2,
                        None,
                    )
                    .expect("Rust SIMD MIN indicator failed");
                black_box(&result_4_2);*/

                // Process remaining 1 option with scalar
                /*let options_1 = [&OPTIONS_LIST[8]];
                let result_1 =
                    tulip_rs::indicators::nightly::by_option::min::indicator_by_option::<1>(
                        &inputs, &options_1, None,
                    )
                    .expect("Rust SIMD MIN indicator failed");
                black_box(&result_1);*/
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_min_simd_by_assets,
    bench_rust_min_simd_by_options,
    bench_rust_min,
    bench_c_min,
    bench_talib_min,
    bench_rust_min_from_state,
);
criterion_main!(benches);
