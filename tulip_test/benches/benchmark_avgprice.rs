use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tulip_rs::indicators::avgprice::{
    indicator, indicator_by_assets, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_avgprice, ti_avgprice_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
use tulip_test::talib_bindings::{ta_avgprice, ta_avgprice_start};

// Sample input data (open, high, low, and close prices)
const OPEN: [f64; 15] = [
    81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
    88.00, 87.60,
];
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

// Options for AVGPRICE (no options needed)
const OPTIONS_LIST: [[f64; 0]; 1] = [[]];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it.
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open_vec = OPEN.to_vec();
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..500 {
        open_vec.extend_from_slice(&OPEN);
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
    }
    (open_vec, high_vec, low_vec, close_vec)
}

/// Benchmark the C implementation of AVGPRICE.
fn bench_c_avgprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("avgprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let open_vec: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs: Vec<*const f64> = vec![
                open_vec.as_ptr(),
                high_vec.as_ptr(),
                low_vec.as_ptr(),
                close_vec.as_ptr(),
            ];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_avgprice_start(options.as_ptr()) };
                        assert!(
                            start_index >= 0,
                            "ti_avgprice_start returned a negative index"
                        );
                        let output_len = open_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_avgprice(
                                open_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_avgprice returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "avgprice",
                    "C_tulip",
                    &options,
                    open_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![
            open_vec.as_ptr(),
            high_vec.as_ptr(),
            low_vec.as_ptr(),
            close_vec.as_ptr(),
        ];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_avgprice_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_avgprice_start returned a negative index"
            );
            let output_len = open_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("C AVGPRICE");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("C AVGPRICE", |b| {
                b.iter(|| {
                    // Allocate output buffer for AVGPRICE.
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_avgprice(
                            open_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_avgprice returned error code {}", ret);
                    black_box(&output_vec);
                })
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of AVGPRICE.
fn bench_rust_avgprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("avgprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let open_vec: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [
                open_vec.as_slice(),
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &options, None)
                            .expect("Rust AVGPRICE indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "avgprice",
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
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            open_vec.as_slice(),
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("Rust AVGPRICE");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("Rust AVGPRICE", |b| {
                b.iter(|| {
                    let result =
                        indicator(&inputs, &options, None).expect("Rust AVGPRICE indicator failed");
                    black_box(&result);
                })
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of AVGPRICE.
fn bench_rust_avgprice_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("avgprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let open_vec: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = open_vec.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [
                            &open_vec[..min_data_val],
                            &high_vec[..min_data_val],
                            &low_vec[..min_data_val],
                            &close_vec[..min_data_val],
                        ];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("AVGPRICE indicator failed");

                        // Chunks
                        let mut open_chunks = open_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (((open_chunk, high_chunk), low_chunk), close_chunk) in open_chunks
                            .by_ref()
                            .zip(high_chunks.by_ref())
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let chunk_inputs = [open_chunk, high_chunk, low_chunk, close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
                        let open_rem = open_chunks.remainder();
                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        let close_rem = close_chunks.remainder();

                        if !open_rem.is_empty()
                            && !high_rem.is_empty()
                            && !low_rem.is_empty()
                            && !close_rem.is_empty()
                        {
                            let chunk_inputs = [open_rem, high_rem, low_rem, close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "avgprice",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if open_vec.len() > 1 {
                    let new_inputs = [
                        &open_vec[..open_vec.len() - 1],
                        &high_vec[..high_vec.len() - 1],
                        &low_vec[..low_vec.len() - 1],
                        &close_vec[..close_vec.len() - 1],
                    ];
                    let final_inputs = [
                        &open_vec[open_vec.len() - 1..],
                        &high_vec[high_vec.len() - 1..],
                        &low_vec[low_vec.len() - 1..],
                        &close_vec[close_vec.len() - 1..],
                    ];
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust AVGPRICE indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust AVGPRICE from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "avgprice",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(&stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust AVGPRICE indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust AVGPRICE from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "avgprice",
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
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();
        let _inputs = [&open_vec, &high_vec, &low_vec, &close_vec];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("Rust AVGPRICE from state");
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = min_data(&options).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &open_vec[..min_data_val],
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                    ];

                    let (_, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("AVGPRICE indicator failed");

                    // Chunks
                    let mut open_chunks = open_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (((open_chunk, high_chunk), low_chunk), close_chunk) in open_chunks
                        .by_ref()
                        .zip(high_chunks.by_ref())
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_inputs = [open_chunk, high_chunk, low_chunk, close_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let open_rem = open_chunks.remainder();
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();

                    if !open_rem.is_empty()
                        && !high_rem.is_empty()
                        && !low_rem.is_empty()
                        && !close_rem.is_empty()
                    {
                        let chunk_inputs = [open_rem, high_rem, low_rem, close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if open_vec.len() > 1 {
                let new_inputs = [
                    &open_vec[..open_vec.len() - 1],
                    &high_vec[..high_vec.len() - 1],
                    &low_vec[..low_vec.len() - 1],
                    &close_vec[..close_vec.len() - 1],
                ];
                let final_inputs = [
                    &open_vec[open_vec.len() - 1..],
                    &high_vec[high_vec.len() - 1..],
                    &low_vec[low_vec.len() - 1..],
                    &close_vec[close_vec.len() - 1..],
                ];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust AVGPRICE indicator failed");

                let mut group = c.benchmark_group("Rust AVGPRICE from state 1 bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust AVGPRICE from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of AVGPRICE.
fn bench_rust_avgprice_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("avgprice");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                (
                    symbol.clone(),
                    data.iter().map(|d| d.open).collect(),
                    data.iter().map(|d| d.high).collect(),
                    data.iter().map(|d| d.low).collect(),
                    data.iter().map(|d| d.close).collect(),
                )
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1, // open
                &stock_data[0].2, // high
                &stock_data[0].3, // low
                &stock_data[0].4, // close
            ],
            &[
                &stock_data[1].1, // open
                &stock_data[1].2, // high
                &stock_data[1].3, // low
                &stock_data[1].4, // close
            ],
            &[
                &stock_data[2].1, // open
                &stock_data[2].2, // high
                &stock_data[2].3, // low
                &stock_data[2].4, // close
            ],
            &[
                &stock_data[3].1, // open
                &stock_data[3].2, // high
                &stock_data[3].3, // low
                &stock_data[3].4, // close
            ],
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets AVGPRICE indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "avgprice",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 4]; 4] = [
            &[&open_vec, &high_vec, &low_vec, &close_vec],
            &[&open_vec, &high_vec, &low_vec, &close_vec],
            &[&open_vec, &high_vec, &low_vec, &close_vec],
            &[&open_vec, &high_vec, &low_vec, &close_vec],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("avgprice_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("Rust SIMD by assets AVGPRICE", |b| {
                b.iter(|| {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets AVGPRICE indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the TA-Lib implementation of AVGPRICE.
fn bench_talib_avgprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("avgprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let open: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = open.len();
            let inputs: Vec<*const f64> =
                vec![open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr()];

            let mut timing = TimingMeasurements::new();

            timing.measure(
                || {
                    let start_index = ta_avgprice_start();
                    assert!(
                        start_index >= 0,
                        "ta_avgprice_start returned a negative index"
                    );
                    let output_len = open.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = ta_avgprice(
                        open.len() as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_avgprice returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("avgprice", "talib", &[], n, &timing, Some(&stock_symbol));
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![
            open_vec.as_ptr(),
            high_vec.as_ptr(),
            low_vec.as_ptr(),
            close_vec.as_ptr(),
        ];

        let start_index = ta_avgprice_start();
        assert!(
            start_index >= 0,
            "ta_avgprice_start returned a negative index"
        );
        let output_len = open_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("avgprice_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib AVGPRICE", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = ta_avgprice(
                    open_vec.len() as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_avgprice returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_avgprice_simd_by_assets,
    bench_rust_avgprice,
    bench_c_avgprice,
    bench_talib_avgprice,
    bench_rust_avgprice_from_state
);
criterion_main!(benches);
