use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::bbands::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_bbands, ti_bbands_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_bbands, ta_bbands_start};

// Sample input data from bbands_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for BBANDS (period, standard_deviation)
const OPTIONS_LIST: [[f64; 2]; 4] = [[5.0, 2.0], [14.0, 2.0], [20.0, 2.0], [50.0, 2.0]];

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

/// Benchmark the C implementation of BBANDS.
fn bench_c_bbands(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_bbands_start(options.as_ptr()) };
                        assert!(
                            start_index >= 0,
                            "ti_bbands_start returned a negative index"
                        );
                        let output_len = close.len() - (start_index as usize);
                        let mut lower_output = vec![0.0_f64; output_len];
                        let mut middle_output = vec![0.0_f64; output_len];
                        let mut upper_output = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![
                            lower_output.as_mut_ptr(),
                            middle_output.as_mut_ptr(),
                            upper_output.as_mut_ptr(),
                        ];

                        let ret = unsafe {
                            ti_bbands(
                                close.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_bbands returned error code {}", ret);
                        black_box(&lower_output);
                        black_box(&middle_output);
                        black_box(&upper_output);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "bbands",
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
            let start_index = unsafe { ti_bbands_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_bbands_start returned a negative index"
            );
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("bbands_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("C BBANDS {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut lower_output = vec![0.0_f64; output_len];
                        let mut middle_output = vec![0.0_f64; output_len];
                        let mut upper_output = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![
                            lower_output.as_mut_ptr(),
                            middle_output.as_mut_ptr(),
                            upper_output.as_mut_ptr(),
                        ];

                        let ret = unsafe {
                            ti_bbands(
                                close_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_bbands returned error code {}", ret);
                        black_box(&lower_output);
                        black_box(&middle_output);
                        black_box(&upper_output);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of BBANDS.
fn bench_rust_bbands(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let _result = indicator(&inputs, &options, None)
                            .expect("Rust BBANDS indicator failed");
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("bbands", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("bbands_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust BBANDS {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust BBANDS indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of BBANDS.
fn bench_rust_bbands_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("bbands");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options);
                        // First chunk
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("BBANDS indicator failed");

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
                    "bbands",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [&close[..close.len() - 1]];
                    let final_inputs = [&close[close.len() - 1..]];
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust BBANDS indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust BBANDS from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "bbands",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust BBANDS indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust BBANDS from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "bbands",
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
            let min_data = min_data(&options);
            // First chunk
            let chunk_inputs = [&close_vec[..min_data]];

            let (_, mut state) =
                indicator(&chunk_inputs, &options, None).expect("BBANDS indicator failed");

            let mut group = c.benchmark_group("bbands_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust BBANDS from state {{ {:?} }}", options),
                |b| {
                    b.iter(|| {
                        let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);

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
                },
            );
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_inputs = [&close_vec[..close_vec.len() - 1]];
                let final_inputs = [&close_vec[close_vec.len() - 1..]];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust BBANDS indicator failed");

                let mut group = c.benchmark_group("bbands_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!("Rust BBANDS from state 1 bar {{ {:?} }}", options),
                    |b| {
                        b.iter(|| {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust BBANDS from state indicator failed");
                            black_box(&result);
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of BBANDS.
fn bench_rust_bbands_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

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
                        .expect("Rust SIMD by assets BBANDS indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "bbands",
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
            let mut group = c.benchmark_group("bbands_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust SIMD by assets BBANDS {{ {}, {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets BBANDS indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by options implementation of BBANDS.
fn bench_rust_bbands_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

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
                        .expect("Rust SIMD BBANDS indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "bbands",
                "Rust_SIMD",
                &[0.0, 0.0],
                close_vec.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("bbands_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options BBANDS (4 lanes)", |b| {
            b.iter(|| {
                // Process all 4 options with 4-wide SIMD
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD BBANDS indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the TA-Lib implementation of BBANDS.
#[cfg(feature = "talib")]
fn bench_talib_bbands(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_bbands_start(options[0]);
                        assert!(
                            start_index >= 0,
                            "ta_bbands_start returned a negative index"
                        );
                        let output_len = close.len() - (start_index as usize);
                        let mut upper_vec = vec![0.0_f64; output_len];
                        let mut middle_vec = vec![0.0_f64; output_len];
                        let mut lower_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![
                            upper_vec.as_mut_ptr(),
                            middle_vec.as_mut_ptr(),
                            lower_vec.as_mut_ptr(),
                        ];
                        let ret = ta_bbands(
                            close.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_bbands returned error code {}", ret);
                        black_box(&upper_vec);
                        black_box(&middle_vec);
                        black_box(&lower_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("bbands", "talib", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_bbands_start(options[0]);
            assert!(
                start_index >= 0,
                "ta_bbands_start returned a negative index"
            );
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("bbands_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("TA-Lib BBANDS {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut upper_vec = vec![0.0_f64; output_len];
                        let mut middle_vec = vec![0.0_f64; output_len];
                        let mut lower_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![
                            upper_vec.as_mut_ptr(),
                            middle_vec.as_mut_ptr(),
                            lower_vec.as_mut_ptr(),
                        ];

                        let ret = ta_bbands(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_bbands returned error code {}", ret);
                        black_box(&upper_vec);
                        black_box(&middle_vec);
                        black_box(&lower_vec);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the `ta` crate (RustTa) implementation of BBands.
fn bench_rust_ta_bbands(c: &mut Criterion) {
    use ta::indicators::BollingerBands;
    use ta::Next;

    if should_log_to_db() {
        init_database_data();
        init_logging("bbands");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                let period = options[0] as usize;
                let multiplier = options[1];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut bb =
                            BollingerBands::new(period, multiplier).expect("ta BBands new failed");
                        let mut last = 0.0_f64;
                        let mut upper = 0.0_f64;
                        let mut lower = 0.0_f64;
                        for &price in &close {
                            let out = bb.next(price);
                            
                            last = out.average;
                            upper = out.upper;
                            lower = out.lower;
                        }
                        black_box(last);
                        black_box(upper);
                        black_box(lower);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "bbands",
                    "RustTa",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let period = options[0] as usize;
            let multiplier = options[1];
            let mut group = c.benchmark_group("bbands_rust_ta");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("RustTa BBands {{ {}/{} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut bb =
                            BollingerBands::new(period, multiplier).expect("ta BBands new failed");
                        let mut last = 0.0_f64;
                        let mut upper = 0.0_f64;
                        let mut lower = 0.0_f64;
                        for &price in &close_vec {
                            let out = bb.next(price);
                            
                            last = out.average;
                            upper = out.upper;
                            lower = out.lower;
                        }
                        black_box(last);
                        black_box(upper);
                        black_box(lower);
                    });
                },
            );
            group.finish();
        }
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_bbands_simd_by_assets,
    bench_rust_bbands_simd_by_options,
    bench_rust_bbands,
    bench_rust_ta_bbands,
    bench_c_bbands,
    bench_talib_bbands,
    bench_rust_bbands_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_bbands_simd_by_assets,
    bench_rust_bbands_simd_by_options,
    bench_rust_bbands,
    bench_rust_ta_bbands,
    bench_c_bbands,
    bench_rust_bbands_from_state,
);
criterion_main!(benches);
