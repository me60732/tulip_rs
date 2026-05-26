use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::pvi::{
    indicator, indicator_by_assets, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_pvi, ti_pvi_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data from pvi_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];
const VOLUME: [f64; 15] = [
    5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
    4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
];

// Options for PVI (no options)
const OPTIONS: [f64; 0] = [];

/// Chunk size for from-state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut close_vec = CLOSE.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..500 {
        close_vec.extend_from_slice(&CLOSE);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (close_vec, volume_vec)
}

/// Benchmark the C implementation of PVI.
fn bench_c_pvi(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("pvi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs: Vec<*const f64> = vec![close_vec.as_ptr(), volume_vec.as_ptr()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_pvi_start(OPTIONS.as_ptr()) };
                    assert!(start_index >= 0, "ti_pvi_start returned a negative index");
                    let output_len = close_vec.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_pvi(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            OPTIONS.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_pvi returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "pvi",
                "C_tulip",
                &OPTIONS,
                close_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr(), volume_vec.as_ptr()];

        let start_index = unsafe { ti_pvi_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_pvi_start returned a negative index");
        let output_len = close_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("pvi_c");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C PVI", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_pvi(
                        close_vec.len() as i32,
                        inputs.as_ptr(),
                        OPTIONS.as_ptr(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_pvi returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust implementation of PVI.
fn bench_rust_pvi(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("pvi");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result =
                        indicator(&inputs, &OPTIONS, None).expect("Rust PVI indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "pvi",
                "Rust",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

        let mut group = c.benchmark_group("pvi_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust PVI", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &OPTIONS, None).expect("Rust PVI indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust from_state implementation of PVI.
fn bench_rust_pvi_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("pvi");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let volume_vec: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
            let n = close_vec.len();
            let inputs = [close_vec.as_slice(), volume_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data = min_data(&OPTIONS).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&close_vec[..min_data], &volume_vec[..min_data]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &OPTIONS, None).expect("PVI indicator failed");

                    // Chunks
                    let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume_vec[min_data..].chunks_exact(CHUNK_SIZE);

                    for (close_chunk, volume_chunk) in
                        close_chunks.by_ref().zip(volume_chunks.by_ref())
                    {
                        let chunk_inputs = [close_chunk, volume_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let close_rem = close_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !close_rem.is_empty() {
                        let chunk_inputs = [close_rem, volume_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "pvi",
                "Rust_FromState",
                &OPTIONS,
                n,
                &timing,
                Some(&stock_symbol),
            );

            // --- Rust_FromState_1_Bar benchmark ---
            if inputs[0].len() > 1 {
                let new_inputs = [
                    &close_vec[..close_vec.len() - 1],
                    &volume_vec[..volume_vec.len() - 1],
                ];
                let final_inputs = [
                    &close_vec[close_vec.len() - 1..],
                    &volume_vec[volume_vec.len() - 1..],
                ];
                let (_, mut state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust PVI indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust PVI from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "pvi",
                    "Rust_FromState_1_Bar",
                    &OPTIONS,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar_json benchmark ---
                let (_, state) =
                    indicator(&new_inputs, &OPTIONS, None).expect("Rust PVI indicator failed");

                let mut timing = TimingMeasurements::new();
                let json = serde_json::to_string(&state).expect("json failed");
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust PVI from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "pvi",
                    "Rust_FromState_1_Bar_json",
                    &OPTIONS,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (close_vec, volume_vec) = expand_inputs();
        let _inputs = [&close_vec, &volume_vec];

        let mut group = c.benchmark_group("Rust PVI from state");
        group.sample_size(SAMPLE_SIZE);

        group.bench_function("benchmark", |b| {
            b.iter(|| {
                let min_data = min_data(&OPTIONS).max(CHUNK_SIZE);
                // First chunk
                let chunk_inputs = [&close_vec[..min_data], &volume_vec[..min_data]];

                let (_, mut state) =
                    indicator(&chunk_inputs, &OPTIONS, None).expect("PVI indicator failed");

                // Chunks
                let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume_vec[min_data..].chunks_exact(CHUNK_SIZE);

                for (close_chunk, volume_chunk) in close_chunks.by_ref().zip(volume_chunks.by_ref())
                {
                    let chunk_inputs = [close_chunk, volume_chunk];
                    let result = state.batch_indicator(&chunk_inputs, None);
                    black_box(&result);
                }

                // Remainder
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();

                if !close_rem.is_empty() {
                    let chunk_inputs = [close_rem, volume_rem];
                    let result = state.batch_indicator(&chunk_inputs, None);
                    black_box(&result);
                }
            });
        });
        group.finish();

        // Benchmark with 1 bar from state
        if close_vec.len() > 1 {
            let new_inputs = [
                &close_vec[..close_vec.len() - 1],
                &volume_vec[..volume_vec.len() - 1],
            ];
            let final_inputs = [
                &close_vec[close_vec.len() - 1..],
                &volume_vec[volume_vec.len() - 1..],
            ];
            let (_, mut state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust PVI indicator failed");

            let mut group = c.benchmark_group("Rust PVI from state 1 bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust PVI from state indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of PVI.
fn bench_rust_pvi_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("pvi");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close and volume data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let close = data.iter().map(|d| d.close).collect();
                let volume = data.iter().map(|d| d.volume).collect();
                (symbol.clone(), close, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2],
            &[&stock_data[1].1, &stock_data[1].2],
            &[&stock_data[2].1, &stock_data[2].2],
            &[&stock_data[3].1, &stock_data[3].2],
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                    .expect("Rust SIMD by assets PVI indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "pvi",
            "Rust_SIMD_by_assets",
            &OPTIONS,
            stock_data[0].1.len(),
            &timing,
            Some("All"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (close_vec, volume_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 2]; 4] = [
            &[close_vec.as_slice(), volume_vec.as_slice()],
            &[close_vec.as_slice(), volume_vec.as_slice()],
            &[close_vec.as_slice(), volume_vec.as_slice()],
            &[close_vec.as_slice(), volume_vec.as_slice()],
        ];

        let mut group = c.benchmark_group("pvi_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by assets PVI", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                    .expect("Rust SIMD by assets PVI indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_pvi_simd_by_assets,
    bench_rust_pvi,
    bench_rust_pvi_from_state,
    bench_c_pvi,
);
criterion_main!(benches);
