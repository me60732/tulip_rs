use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::ad::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_ad, ti_ad_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ad, ta_ad_start};

// Sample input data from ad_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];
const VOLUME: [f64; 15] = [
    5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
    4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
];

// Options for AD (no options)
const OPTIONS: [f64; 0] = [];

// Chunk size for from_state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..200 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (high_vec, low_vec, close_vec, volume_vec)
}

/// Benchmark the C implementation of AD.
fn bench_c_ad(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ad");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs: Vec<*const f64> = vec![
                high_vec.as_ptr(),
                low_vec.as_ptr(),
                close_vec.as_ptr(),
                volume_vec.as_ptr(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_ad_start(OPTIONS.as_ptr()) };
                    //assert!(start_index >= 0, "ti_ad_start returned a negative index");
                    let output_len = high_vec.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_ad(
                            high_vec.len() as i32,
                            inputs.as_ptr(),
                            OPTIONS.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_ad returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ad",
                "C_tulip",
                &OPTIONS,
                high_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![
            high_vec.as_ptr(),
            low_vec.as_ptr(),
            close_vec.as_ptr(),
            volume_vec.as_ptr(),
        ];

        let start_index = unsafe { ti_ad_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_ad_start returned a negative index");
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("ad_c");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C AD", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_ad(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        OPTIONS.as_ptr(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_ad returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust implementation of AD.
fn bench_rust_ad(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ad");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
                volume_vec.as_slice(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result =
                        indicator(&inputs, &OPTIONS, None).expect("Rust AD indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ad",
                "Rust",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
            volume_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("ad_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust AD", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &OPTIONS, None).expect("Rust AD indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust from_state implementation of AD.
fn bench_rust_ad_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ad");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();

            let new_inputs = [
                &high_vec[..high_vec.len() - 1],
                &low_vec[..low_vec.len() - 1],
                &close_vec[..close_vec.len() - 1],
                &volume_vec[..volume_vec.len() - 1],
            ];
            let final_inputs = [
                &high_vec[high_vec.len() - 1..],
                &low_vec[low_vec.len() - 1..],
                &close_vec[close_vec.len() - 1..],
                &volume_vec[volume_vec.len() - 1..],
            ];

            let inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
                volume_vec.as_slice(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                        &volume_vec[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &OPTIONS, None).expect("Rust AD indicator failed");

                    // Chunks
                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (((high_chunk, low_chunk), close_chunk), volume_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                        .zip(volume_chunks.by_ref())
                    {
                        let chunk_inputs = [high_chunk, low_chunk, close_chunk, volume_chunk];

                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ad",
                "Rust_FromState",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );

            // --- Rust_FromState_1_Bar benchmark ---
            if inputs[0].len() > 1 {
                let (_, mut state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust AD indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust AD from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ad",
                    "Rust_FromState_1_Bar",
                    &OPTIONS,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_symbol),
                );

                let (_, state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust AD indicator failed");
                let json = serde_json::to_string(&state).expect("json failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust AD from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ad",
                    "Rust_FromState_1_Bar_json",
                    &OPTIONS,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();

        let new_inputs = [
            &high_vec[..high_vec.len() - 1],
            &low_vec[..low_vec.len() - 1],
            &close_vec[..close_vec.len() - 1],
            &volume_vec[..volume_vec.len() - 1],
        ];
        let final_inputs = [
            &high_vec[high_vec.len() - 1..],
            &low_vec[low_vec.len() - 1..],
            &close_vec[close_vec.len() - 1..],
            &volume_vec[volume_vec.len() - 1..],
        ];

        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
            volume_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("ad_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust AD from state", |b| {
            b.iter(|| {
                let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
                // First chunk
                let chunk_inputs = [
                    &high_vec[..min_data_val],
                    &low_vec[..min_data_val],
                    &close_vec[..min_data_val],
                    &volume_vec[..min_data_val],
                ];

                let (_, mut state) =
                    indicator(&chunk_inputs, &OPTIONS, None).expect("Rust AD indicator failed");

                // Chunks
                let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (((high_chunk, low_chunk), close_chunk), volume_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let chunk_inputs = [high_chunk, low_chunk, close_chunk, volume_chunk];
                    let result = state.batch_indicator(&chunk_inputs, None);
                    black_box(&result);
                }

                // Remainder
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();

                if !high_rem.is_empty() {
                    let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
                    let result = state.batch_indicator(&chunk_inputs, None);
                    black_box(&result);
                }
            });
        });

        // --- Rust_FromState_1_Bar benchmark ---
        if inputs[0].len() > 1 {
            let (_, mut state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust AD indicator failed");

            group.bench_function("Rust AD from state 1 bar", |b| {
                b.iter(|| {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust AD from state indicator failed");
                    black_box(&result);
                });
            });
        }
        group.finish();
    }
}

/// Benchmark the TA-Lib implementation of AD.
#[cfg(feature = "talib")]
fn bench_talib_ad(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ad");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let n = high.len();
            let inputs: Vec<*const f64> =
                vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

            let mut timing = TimingMeasurements::new();

            timing.measure(
                || {
                    let start_index = ta_ad_start();
                    assert!(start_index >= 0, "ta_ad_start returned a negative index");
                    let output_len = high.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = ta_ad(
                        high.len() as i32,
                        inputs.as_ptr(),
                        OPTIONS.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_ad returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("ad", "talib", &OPTIONS, n, &timing, Some(&stock_symbol));
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![
            high_vec.as_ptr(),
            low_vec.as_ptr(),
            close_vec.as_ptr(),
            volume_vec.as_ptr(),
        ];

        let start_index = ta_ad_start();
        assert!(start_index >= 0, "ta_ad_start returned a negative index");
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("ad_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib AD", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = ta_ad(
                    high_vec.len() as i32,
                    inputs.as_ptr(),
                    OPTIONS.as_ptr(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_ad returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}
/// Benchmark the Rust SIMD by assets implementation of AD.
fn bench_rust_ad_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ad");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                (
                    symbol.clone(),
                    data.iter().map(|d| d.high).collect(),
                    data.iter().map(|d| d.low).collect(),
                    data.iter().map(|d| d.close).collect(),
                    data.iter().map(|d| d.volume).collect(),
                )
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // close
                &stock_data[0].4, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // close
                &stock_data[1].4, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // close
                &stock_data[2].4, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // close
                &stock_data[3].4, // volume
            ],
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    tulip_rs::indicators::ad::indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                        .expect("Rust SIMD by assets AD indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "ad",
            "Rust_SIMD_by_assets",
            &OPTIONS,
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 4]; 4] = [
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
        ];

        let mut group = c.benchmark_group("ad_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by assets AD", |b| {
            b.iter(|| {
                let result =
                    tulip_rs::indicators::ad::indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                        .expect("Rust SIMD by assets AD indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_ad_simd_by_assets,
    bench_rust_ad,
    bench_c_ad,
    bench_talib_ad,
    bench_rust_ad_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_ad_simd_by_assets,
    bench_rust_ad,
    bench_c_ad,
    bench_rust_ad_from_state,
);
criterion_main!(benches);
