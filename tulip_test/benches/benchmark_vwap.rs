use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::vwap::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{eod_data_to_arrays, get_all_stock_data, init_database_data};

// Sample input data (high, low, close prices and volume)
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
const VOLUME: [f64; 15] = [
    5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
    4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
];

// Options for VWAP (no options)
const OPTIONS: [f64; 0] = [];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (high_vec, low_vec, close_vec, volume_vec)
}

/// Benchmark the Rust implementation of VWAP.
fn bench_rust_vwap(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwap");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high_vec, low_vec, close_vec, volume_vec) = eod_data_to_arrays(stock_data);
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
                        indicator(&inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "vwap",
                "Rust",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(stock_symbol),
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

        let mut group = c.benchmark_group("vwap_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust VWAP", |b| {
            b.iter(|| {
                let result =
                    indicator(&inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust from_state implementation of VWAP.
fn bench_rust_vwap_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwap");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high_vec, low_vec, close_vec, volume_vec) = eod_data_to_arrays(stock_data);
            let n = high_vec.len();

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
                        indicator(&chunk_inputs, &OPTIONS, None).expect("VWAP indicator failed");

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

                    if !high_rem.is_empty()
                        && !low_rem.is_empty()
                        && !close_rem.is_empty()
                        && !volume_rem.is_empty()
                    {
                        let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "vwap",
                "Rust_from_state",
                &OPTIONS,
                n,
                &timing,
                Some(stock_symbol),
            );

            // --- Rust_from_state_1_Bar benchmark ---
            if high_vec.len() > 1 {
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
                let (_, mut state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust VWAP from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwap",
                    "Rust_from_state_1_Bar",
                    &OPTIONS,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_from_state_1_Bar_json benchmark ---
                let (_, state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");
                let json = serde_json::to_string(&state).expect("json failed");
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust VWAP from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "vwap",
                    "Rust_from_state_1_Bar_json",
                    &OPTIONS,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();

        let mut group = c.benchmark_group("Rust VWAP from state");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("benchmark", |b| {
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
                    indicator(&chunk_inputs, &OPTIONS, None).expect("VWAP indicator failed");

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

                if !high_rem.is_empty()
                    && !low_rem.is_empty()
                    && !close_rem.is_empty()
                    && !volume_rem.is_empty()
                {
                    let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
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
                &volume_vec[..volume_vec.len() - 1],
            ];
            let final_inputs = [
                &high_vec[high_vec.len() - 1..],
                &low_vec[low_vec.len() - 1..],
                &close_vec[close_vec.len() - 1..],
                &volume_vec[volume_vec.len() - 1..],
            ];
            let (_, mut state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");

            let mut group = c.benchmark_group("Rust VWAP from state 1 bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust VWAP from state indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by-assets implementation of VWAP.
fn bench_rust_vwap_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("vwap");

        let data = get_all_stock_data().unwrap();

        // First 4 stocks — one per SIMD lane.
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (h, l, c, v) = eod_data_to_arrays(eod);
                (symbol.clone(), h, l, c, v)
            })
            .collect();

        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1,
                &stock_data[0].2,
                &stock_data[0].3,
                &stock_data[0].4,
            ],
            &[
                &stock_data[1].1,
                &stock_data[1].2,
                &stock_data[1].3,
                &stock_data[1].4,
            ],
            &[
                &stock_data[2].1,
                &stock_data[2].2,
                &stock_data[2].3,
                &stock_data[2].4,
            ],
            &[
                &stock_data[3].1,
                &stock_data[3].2,
                &stock_data[3].3,
                &stock_data[3].4,
            ],
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    tulip_rs::indicators::vwap::indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                        .expect("Rust SIMD by-assets VWAP failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "vwap",
            "Rust_SIMD_by_assets",
            &OPTIONS,
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();

        // Four identical lanes for the synthetic benchmark.
        let inputs: [&[&[f64]; 4]; 4] = [
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
            &[&high_vec, &low_vec, &close_vec, &volume_vec],
        ];

        let mut group = c.benchmark_group("vwap_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by-assets VWAP", |b| {
            b.iter(|| {
                let result =
                    tulip_rs::indicators::vwap::indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                        .expect("Rust SIMD by-assets VWAP failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_vwap_simd_by_assets,
    bench_rust_vwap,
    bench_rust_vwap_from_state,
);
criterion_main!(benches);
