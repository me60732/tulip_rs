use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::marketfi::{
    indicator, indicator_by_assets, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_marketfi, ti_marketfi_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Test input data (high, low, volume prices) - copied from test file
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

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for profiling
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (high_vec, low_vec, volume_vec)
}

// Helper function to get HLV arrays from stock data
fn get_hlv_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
    (high, low, volume)
}

fn bench_c_marketfi(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("marketfi");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_marketfi_start(std::ptr::null()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = unsafe {
                        ti_marketfi(
                            high.len() as i32,
                            inputs.as_ptr(),
                            std::ptr::null(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_marketfi returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );
            log_timing_result("marketfi", "C_tulip", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let (high_vec, low_vec, volume_vec) = expand_inputs();
        let inputs: Vec<*const f64> =
            vec![high_vec.as_ptr(), low_vec.as_ptr(), volume_vec.as_ptr()];

        let start_index = unsafe { ti_marketfi_start(std::ptr::null()) };
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("C MARKETFI");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C MARKETFI", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_marketfi(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_marketfi returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

fn bench_rust_marketfi(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("marketfi");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&inputs, &[], None).expect("MARKETFI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result("marketfi", "Rust", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let (high_vec, low_vec, volume_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            volume_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("Rust MARKETFI");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust MARKETFI", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &[], None).expect("MARKETFI indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

fn bench_rust_marketfi_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("marketfi");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(stock_data);
            let n = high.len();

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data_val = min_data(&[]).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &volume[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &[], None).expect("MARKETFI indicator failed");

                    // Chunks
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), volume_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(volume_chunks.by_ref())
                    {
                        let result =
                            state.batch_indicator(&[high_chunk, low_chunk, volume_chunk], None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !high_rem.is_empty() && !low_rem.is_empty() && !volume_rem.is_empty() {
                        let result = state.batch_indicator(&[high_rem, low_rem, volume_rem], None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "marketfi",
                "Rust_FromState",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );

            // --- Rust_FromState_1_Bar benchmark ---
            if high.len() > 1 {
                let new_inputs = [
                    &high[..high.len() - 1],
                    &low[..low.len() - 1],
                    &volume[..volume.len() - 1],
                ];

                let final_inputs = [
                    &high[high.len() - 1..],
                    &low[low.len() - 1..],
                    &volume[volume.len() - 1..],
                ];
                let (_, mut state) =
                    indicator(&new_inputs, &[], None).expect("Rust MARKETFI indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust MARKETFI from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "marketfi",
                    "Rust_FromState_1_Bar",
                    &[],
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar_json benchmark ---
                let (_, state) =
                    indicator(&new_inputs, &[], None).expect("Rust MARKETFI indicator failed");
                let json = serde_json::to_string(&state).expect("json failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust MARKETFI from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "marketfi",
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
        let (high_vec, low_vec, volume_vec) = expand_inputs();
        let _inputs = [&high_vec, &low_vec, &volume_vec];

        let mut group = c.benchmark_group("Rust MARKETFI from state");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("benchmark", |b| {
            b.iter(|| {
                let min_data_val = min_data(&[]).max(CHUNK_SIZE);
                // First chunk
                let chunk_inputs = [
                    &high_vec[..min_data_val],
                    &low_vec[..min_data_val],
                    &volume_vec[..min_data_val],
                ];

                let (_, mut state) =
                    indicator(&chunk_inputs, &[], None).expect("MARKETFI indicator failed");

                // Chunks
                let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                for ((high_chunk, low_chunk), volume_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let result =
                        state.batch_indicator(&[high_chunk, low_chunk, volume_chunk], None);
                    black_box(&result);
                }

                // Remainder
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let volume_rem = volume_chunks.remainder();

                if !high_rem.is_empty() && !low_rem.is_empty() && !volume_rem.is_empty() {
                    let result = state.batch_indicator(&[high_rem, low_rem, volume_rem], None);
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
                &volume_vec[..volume_vec.len() - 1],
            ];

            let final_inputs = [
                &high_vec[high_vec.len() - 1..],
                &low_vec[low_vec.len() - 1..],
                &volume_vec[volume_vec.len() - 1..],
            ];
            let (_, mut state) =
                indicator(&new_inputs, &[], None).expect("Rust MARKETFI indicator failed");

            let mut group = c.benchmark_group("Rust MARKETFI from state 1 bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust MARKETFI from state indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

fn bench_rust_marketfi_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("marketfi");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, volume) = get_hlv_arrays(data);
                (symbol.clone(), high, low, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3], // high, low, volume
            &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3], // high, low, volume
            &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3], // high, low, volume
            &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3], // high, low, volume
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("Rust SIMD by assets MARKETFI indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "marketfi",
            "Rust_SIMD_by_assets",
            &[],
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, volume_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high_vec, &low_vec, &volume_vec],
            &[&high_vec, &low_vec, &volume_vec],
            &[&high_vec, &low_vec, &volume_vec],
            &[&high_vec, &low_vec, &volume_vec],
        ];

        let mut group = c.benchmark_group("marketfi_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by assets MARKETFI", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("Rust SIMD by assets MARKETFI indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    marketfi_benchmarks,
    bench_rust_marketfi,
    bench_rust_marketfi_from_state,
    bench_rust_marketfi_simd_by_assets,
    bench_c_marketfi,
);
criterion_main!(marketfi_benchmarks);
