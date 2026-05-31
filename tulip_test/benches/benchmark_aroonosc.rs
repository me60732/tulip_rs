use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::aroonosc::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
const SAMPLE_SIZE: usize = 30000;
use tulip_test::c_bindings::{ti_aroonosc, ti_aroonosc_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_aroonosc, ta_aroonosc_start};

// Sample input data from aroonosc_test.rs
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

// Options for AROONOSC (period)
//const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [14.0], [25.0], [35.0], [50.0], [100.0]];
/*const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    [8.0],
    [10.0],
    [14.0],
    [25.0],
    [35.0],
    [50.0],
    [100.0],
];*/
const OPTIONS_LIST: [[f64; 1]; 4] = [[25.0], [35.0], [50.0], [100.0]];
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it.
fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

// Helper function to get HL arrays from stock data
fn get_hl_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    (high, low)
}

/// Benchmark the C implementation of AROONOSC.
fn bench_c_aroonosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_aroonosc_start(options.as_ptr()) };
                        assert!(
                            start_index >= 0,
                            "ti_aroonosc_start returned a negative index"
                        );
                        let output_len = high.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_aroonosc(
                                high.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_aroonosc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "aroonosc",
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
        let (high_vec, low_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_aroonosc_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_aroonosc_start returned a negative index"
            );
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("aroonosc_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("C AROONOSC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_aroonosc(
                            high_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_aroonosc returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of AROONOSC.
fn bench_rust_aroonosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust AROONOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result("aroonosc", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("aroonosc_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust AROONOSC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust AROONOSC indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}
/// Benchmark the Rust from_state implementation of AROONOSC.
fn bench_rust_aroonosc_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options);
                        // First chunk
                        let chunk_inputs = [&high[..min_data], &low[..min_data]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("AROONOSC indicator failed");

                        // Chunks
                        let mut high_chunks = high[min_data..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data..].chunks_exact(CHUNK_SIZE);

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
                    "aroonosc",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [&high[..high.len() - 1], &low[..low.len() - 1]];
                    let final_inputs = [&high[high.len() - 1..], &low[low.len() - 1..]];
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust AROONOSC indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust AROONOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "aroonosc",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust AROONOSC indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust AROONOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "aroonosc",
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
        let (high_vec, low_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group =
                c.benchmark_group(format!("Rust AROONOSC from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data = min_data(&options);
                    // First chunk
                    let chunk_inputs = [&high_vec[..min_data], &low_vec[..min_data]];

                    let (_, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("AROONOSC indicator failed");

                    // Chunks
                    let mut high_chunks = high_vec[min_data..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data..].chunks_exact(CHUNK_SIZE);

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
/// Benchmark the Rust implementation of AROONOSC with optional outputs.
fn bench_rust_aroonosc_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, Some(&[true, true]))
                            .expect("Rust AROONOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "aroonosc",
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
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("aroonosc_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust AROONOSC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &options, Some(&[true, true]))
                        .expect("Rust AROONOSC indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the TA-Lib implementation of AROONOSC.
#[cfg(feature = "talib")]
fn bench_talib_aroonosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_aroonosc_start(options[0]);
                        assert!(
                            start_index >= 0,
                            "ta_aroonosc_start returned a negative index"
                        );
                        let output_len = high.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_aroonosc(
                            high.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_aroonosc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "aroonosc",
                    "talib",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_aroonosc_start(options[0]);
            assert!(
                start_index >= 0,
                "ta_aroonosc_start returned a negative index"
            );
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("aroonosc_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("TA-Lib AROONOSC {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_aroonosc(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_aroonosc returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_aroonosc_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

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

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2], // high, low
            &[&stock_data[1].1, &stock_data[1].2], // high, low
            &[&stock_data[2].1, &stock_data[2].2], // high, low
            &[&stock_data[3].1, &stock_data[3].2], // high, low
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = tulip_rs::indicators::aroonosc::indicator_by_assets::<4>(
                        &inputs, &options, None,
                    )
                    .expect("Rust SIMD by assets AROONOSC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "aroonosc",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&high, &low],
            &[&high, &low],
            &[&high, &low],
            &[&high, &low],
        ];

        for options in OPTIONS_LIST {
            c.bench_function(
                &format!("SIMD by assets AROONOSC {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = tulip_rs::indicators::aroonosc::indicator_by_assets::<4>(
                            &inputs, &options, None,
                        )
                        .expect("Rust SIMD by assets AROONOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
        }
    }
}

fn bench_rust_aroonosc_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];

    if should_log_to_db() {
        init_database_data();
        init_logging("aroonosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = tulip_rs::indicators::aroonosc::indicator_by_options::<4>(
                        &inputs, &options_4, None,
                    )
                    .expect("Rust SIMD AROONOSC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "aroonosc",
                "Rust_SIMD",
                &[0.0],
                high.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice()];

        let mut group = c.benchmark_group("aroonosc_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("Rust SIMD by options AROONOSC (4 lanes)", |b| {
            b.iter(|| {
                let result = tulip_rs::indicators::aroonosc::indicator_by_options::<4>(
                    &inputs, &options_4, None,
                )
                .expect("Rust SIMD AROONOSC indicator failed");
                black_box(&result);
            });
        });

        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_aroonosc_simd_by_assets,
    bench_rust_aroonosc_simd_by_options,
    bench_rust_aroonosc,
    bench_c_aroonosc,
    bench_talib_aroonosc,
    bench_rust_aroonosc_optional,
    bench_rust_aroonosc_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_aroonosc_simd_by_assets,
    bench_rust_aroonosc_simd_by_options,
    bench_rust_aroonosc,
    bench_c_aroonosc,
    bench_rust_aroonosc_optional,
    bench_rust_aroonosc_from_state,
);
criterion_main!(benches);
