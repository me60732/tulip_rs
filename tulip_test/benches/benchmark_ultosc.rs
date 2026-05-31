use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::ultosc::indicator_by_options;
use tulip_rs::indicators::ultosc::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_ultosc, ti_ultosc_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Test input data (high, low, close prices) - copied from test file
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for ULTOSC - copied from test file
const OPTIONS_LIST: [[f64; 3]; 4] = [
    [2.0, 3.0, 5.0],
    [10.0, 14.0, 20.0],
    [14.0, 20.0, 50.0],
    [20.0, 50.0, 100.0],
];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for profiling
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
    }
    (high_vec, low_vec, close_vec)
}

// Helper function to get HLC arrays from stock data
fn get_hlc_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

fn bench_c_ultosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ultosc");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_ultosc_start(options.as_ptr()) };
                        let output_len = high.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_ultosc(
                                high.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_ultosc returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ultosc",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_ultosc_start(options.as_ptr()) };
            let output_len = high_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group(format!(
                "C ULTOSC {{ {:.1}, {:.1}, {:.1} }}",
                options[0], options[1], options[2]
            ));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "C ULTOSC {{ {:.1}, {:.1}, {:.1} }}",
                    options[0], options[1], options[2]
                ),
                |b| {
                    b.iter(|| {
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_ultosc(
                                high_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_ultosc returned error code {}", ret);
                        black_box(&output_vec);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ultosc(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ultosc");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("ULTOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ultosc", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!(
                "Rust ULTOSC {{ {:.1}, {:.1}, {:.1} }}",
                options[0], options[1], options[2]
            ));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust ULTOSC {{ {:.1}, {:.1}, {:.1} }}",
                    options[0], options[1], options[2]
                ),
                |b| {
                    b.iter(|| {
                        let result =
                            indicator(&inputs, &options, None).expect("ULTOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ultosc_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ultosc");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [
                            &high[..min_data_val],
                            &low[..min_data_val],
                            &close[..min_data_val],
                        ];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("ULTOSC indicator failed");

                        // Chunks
                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for ((high_chunk, low_chunk), close_chunk) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let chunk_inputs = [high_chunk, low_chunk, close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        let close_rem = close_chunks.remainder();

                        if !high_rem.is_empty() {
                            let chunk_inputs = [high_rem, low_rem, close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ultosc",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if high.len() > 1 {
                    let new_inputs = [
                        &high[..high.len() - 1],
                        &low[..low.len() - 1],
                        &close[..close.len() - 1],
                    ];
                    let final_inputs = [
                        &high[high.len() - 1..],
                        &low[low.len() - 1..],
                        &close[close.len() - 1..],
                    ];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("ULTOSC indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust ULTOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ultosc",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust ULTOSC indicator failed");
                    let bin = bincode::serde::encode_to_vec(&state, bincode::config::standard())
                        .expect("bincode encode failed");
                    //let json = serde_json::to_string(&state).expect("json failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            /*let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");*/
                            let (mut state, _): (IndicatorState, _) =
                                bincode::serde::decode_from_slice(
                                    &bin,
                                    bincode::config::standard(),
                                )
                                .expect("bincode decode failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust ULTOSC from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "ultosc",
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
        let (high_vec, low_vec, close_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!(
                "Rust ULTOSC from state {{ {:.1}, {:.1}, {:.1} }}",
                options[0], options[1], options[2]
            ));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &options, None).expect("ULTOSC indicator failed");

                    // Chunks
                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), close_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_inputs = [high_chunk, low_chunk, close_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if high_vec.len() > 1 {
                let new_inputs = [
                    &high_vec[..high_vec.len() - 1],
                    &low_vec[..low_vec.len() - 1],
                    &close_vec[..close_vec.len() - 1],
                ];
                let final_inputs = [
                    &high_vec[high_vec.len() - 1..],
                    &low_vec[low_vec.len() - 1..],
                    &close_vec[close_vec.len() - 1..],
                ];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("ULTOSC indicator failed");

                let mut group = c.benchmark_group(format!(
                    "Rust ULTOSC from state 1 bar {{ {:.1}, {:.1}, {:.1} }}",
                    options[0], options[1], options[2]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("ULTOSC from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

fn bench_rust_ultosc_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ultosc");

        let data = get_all_stock_data().unwrap();

        // Process 4 stocks at a time for SIMD
        for chunk in data.chunks(4) {
            if chunk.len() < 4 {
                continue; // Skip incomplete chunks
            }

            let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = chunk
                .iter()
                .map(|(symbol, data)| {
                    let (high, low, close) = get_hlc_arrays(data);
                    (symbol.clone(), high, low, close)
                })
                .collect();

            let inputs: [&[&[f64]; 3]; 4] = [
                &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3],
                &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3],
                &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3],
                &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3],
            ];

            let n = stock_data[0].1.len(); // Use first stock's length

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        use tulip_rs::indicators::ultosc::indicator_by_assets;
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by assets ULTOSC indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ultosc",
                    "Rust_SIMD_by_assets",
                    &options,
                    n,
                    &timing,
                    Some("All"),
                );
            }
        }
    } else {
        // For non-database benchmarks, create 4 copies of expanded inputs
        let (high_vec, low_vec, close_vec) = expand_inputs();

        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!(
                "Rust ULTOSC SIMD by assets {{ {:.1}, {:.1}, {:.1} }}",
                options[0], options[1], options[2]
            ));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust ULTOSC SIMD by assets {{ {:.1}, {:.1}, {:.1} }}",
                    options[0], options[1], options[2]
                ),
                |b| {
                    b.iter(|| {
                        use tulip_rs::indicators::ultosc::indicator_by_assets;
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by assets ULTOSC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_ultosc_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];

    if should_log_to_db() {
        init_database_data();
        init_logging("ultosc");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data.iter().take(4) {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("SIMD by options ULTOSC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ultosc",
                "Rust_SIMD",
                &[0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Lightweight benchmark using synthetic data
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        let mut group = c.benchmark_group("ultosc_simd_by_options_synthetic");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("synthetic_data", |b| {
            b.iter(|| {
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by options ULTOSC indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    ultosc_benchmarks,
    bench_rust_ultosc_simd_by_assets,
    bench_rust_ultosc_simd_by_options,
    bench_rust_ultosc,
    bench_c_ultosc,
    bench_rust_ultosc_from_state,
);
criterion_main!(ultosc_benchmarks);
