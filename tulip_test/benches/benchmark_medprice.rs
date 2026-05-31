use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::medprice::{
    indicator, indicator_by_assets, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_medprice, ti_medprice_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_medprice, ta_medprice_start};

// Test input data (high and low prices) - copied from test file
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for profiling
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

fn bench_c_medprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("medprice");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_medprice_start(std::ptr::null()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_medprice(
                            high.len() as i32,
                            inputs.as_ptr(),
                            std::ptr::null(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_medprice returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );
            log_timing_result("medprice", "C_tulip", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];

        let start_index = unsafe { ti_medprice_start(std::ptr::null()) };
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("C MEDPRICE");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C MEDPRICE", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_medprice(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_medprice returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

fn bench_rust_medprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("medprice");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&inputs, &[], None).expect("MEDPRICE indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result("medprice", "Rust", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        let mut group = c.benchmark_group("Rust MEDPRICE");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust MEDPRICE", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &[], None).expect("MEDPRICE indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

fn bench_rust_medprice_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("medprice");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data_val = min_data(&[]).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&high[..min_data_val], &low[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &[], None).expect("MEDPRICE indicator failed");

                    // Chunks
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let high_chunk_vec = high_chunk.to_vec();
                        let low_chunk_vec = low_chunk.to_vec();
                        let result =
                            state.batch_indicator(&[&high_chunk_vec, &low_chunk_vec], None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();

                    if !high_rem.is_empty() && !low_rem.is_empty() {
                        let high_rem_vec = high_rem.to_vec();
                        let low_rem_vec = low_rem.to_vec();
                        let result = state.batch_indicator(&[&high_rem_vec, &low_rem_vec], None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "medprice",
                "Rust_FromState",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );

            // --- Rust_FromState_1_Bar benchmark ---
            if high.len() > 1 {
                let new_high_vec = high[..high.len() - 1].to_vec();
                let new_low_vec = low[..low.len() - 1].to_vec();
                let new_inputs = [new_high_vec.as_slice(), new_low_vec.as_slice()];

                let final_high_vec = high[high.len() - 1..].to_vec();
                let final_low_vec = low[low.len() - 1..].to_vec();
                let (_, mut state) =
                    indicator(&new_inputs, &[], None).expect("Rust MEDPRICE indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(
                                &[final_high_vec.as_slice(), final_low_vec.as_slice()],
                                None,
                            )
                            .expect("Rust MEDPRICE from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "medprice",
                    "Rust_FromState_1_Bar",
                    &[],
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar_json benchmark ---
                let (_, state) =
                    indicator(&new_inputs, &[], None).expect("Rust MEDPRICE indicator failed");
                let json = serde_json::to_string(&state).expect("json failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(
                                &[final_high_vec.as_slice(), final_low_vec.as_slice()],
                                None,
                            )
                            .expect("Rust MEDPRICE from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "medprice",
                    "Rust_FromState_1_Bar_json",
                    &[],
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let _inputs = [&high_vec, &low_vec];

        let mut group = c.benchmark_group("Rust MEDPRICE from state");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("benchmark", |b| {
            b.iter(|| {
                let min_data_val = min_data(&[]).max(CHUNK_SIZE);
                // First chunk
                let chunk_inputs = [&high_vec[..min_data_val], &low_vec[..min_data_val]];

                let (_, mut state) =
                    indicator(&chunk_inputs, &[], None).expect("MEDPRICE indicator failed");

                // Chunks
                let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                    let high_chunk_vec = high_chunk.to_vec();
                    let low_chunk_vec = low_chunk.to_vec();
                    let result = state.batch_indicator(&[&high_chunk_vec, &low_chunk_vec], None);
                    black_box(&result);
                }

                // Remainder
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();

                if !high_rem.is_empty() && !low_rem.is_empty() {
                    let high_rem_vec = high_rem.to_vec();
                    let low_rem_vec = low_rem.to_vec();
                    let result = state.batch_indicator(&[&high_rem_vec, &low_rem_vec], None);
                    black_box(&result);
                }
            });
        });
        group.finish();

        // Benchmark with 1 bar from state
        if high_vec.len() > 1 {
            let new_high_vec = high_vec[..high_vec.len() - 1].to_vec();
            let new_low_vec = low_vec[..low_vec.len() - 1].to_vec();
            let new_inputs = [new_high_vec.as_slice(), new_low_vec.as_slice()];

            let final_high_vec = high_vec[high_vec.len() - 1..].to_vec();
            let final_low_vec = low_vec[low_vec.len() - 1..].to_vec();
            let (_, mut state) =
                indicator(&new_inputs, &[], None).expect("Rust MEDPRICE indicator failed");

            let mut group = c.benchmark_group("Rust MEDPRICE from state 1 bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let result = state
                        .batch_indicator(
                            &[final_high_vec.as_slice(), final_low_vec.as_slice()],
                            None,
                        )
                        .expect("Rust MEDPRICE from state indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of MEDPRICE.
fn bench_rust_medprice_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("medprice");

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

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("Rust SIMD by assets MEDPRICE indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "medprice",
            "Rust_SIMD_by_assets",
            &[],
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&high_vec, &low_vec],
            &[&high_vec, &low_vec],
            &[&high_vec, &low_vec],
            &[&high_vec, &low_vec],
        ];

        let mut group = c.benchmark_group("medprice_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by assets MEDPRICE", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("Rust SIMD by assets MEDPRICE indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the TA-Lib implementation of MEDPRICE.
#[cfg(feature = "talib")]
fn bench_talib_medprice(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("medprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            let mut timing = TimingMeasurements::new();

            timing.measure(
                || {
                    let start_index = ta_medprice_start();
                    assert!(
                        start_index >= 0,
                        "ta_medprice_start returned a negative index"
                    );
                    let output_len = high.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = ta_medprice(
                        high.len() as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_medprice returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("medprice", "talib", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];

        let start_index = ta_medprice_start();
        assert!(
            start_index >= 0,
            "ta_medprice_start returned a negative index"
        );
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("medprice_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib MEDPRICE", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = ta_medprice(
                    high_vec.len() as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_medprice returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    medprice_benchmarks,
    bench_rust_medprice_simd_by_assets,
    bench_rust_medprice,
    bench_rust_medprice_from_state,
    bench_c_medprice,
    bench_talib_medprice,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    medprice_benchmarks,
    bench_rust_medprice_simd_by_assets,
    bench_rust_medprice,
    bench_rust_medprice_from_state,
    bench_c_medprice,
);
criterion_main!(medprice_benchmarks);
