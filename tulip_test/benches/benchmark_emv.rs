use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::emv::{
    indicator, indicator_by_assets, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_emv, ti_emv_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Test data from emv_test.rs
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

// Options from emv_test.rs
const OPTIONS_LIST: [f64; 0] = [];

// Chunk size for batched processing
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for synthetic benchmarking
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut volume_vec = VOLUME.to_vec();
    for _ in 0..499 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        volume_vec.extend_from_slice(&VOLUME);
    }
    (high_vec, low_vec, volume_vec)
}

/// Extract HLV arrays from stock data
fn get_hlv_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
    (high, low, volume)
}

/// Benchmark the C implementation of EMV.
fn bench_c_emv(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("emv");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_emv_start(OPTIONS_LIST.as_ptr()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = unsafe {
                        ti_emv(
                            high.len() as i32,
                            inputs.as_ptr(),
                            OPTIONS_LIST.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_emv returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "emv",
                "C_tulip",
                &OPTIONS_LIST,
                n,
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, volume) = expand_inputs();

        c.bench_function("benchmark", |b| {
            b.iter(|| {
                let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];
                let start_index = unsafe { ti_emv_start(OPTIONS_LIST.as_ptr()) };
                let output_len = high.len() - (start_index as usize);
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_emv(
                        high.len() as i32,
                        black_box(&inputs).as_ptr(),
                        OPTIONS_LIST.as_ptr(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_emv returned error code {}", ret);
                black_box(&output_vec);
            });
        });
    }
}

/// Benchmark the Rust implementation of EMV.
fn bench_rust_emv(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("emv");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result =
                        indicator(&inputs, &OPTIONS_LIST, None).expect("EMV indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "emv",
                "Rust",
                &OPTIONS_LIST,
                n,
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, volume) = expand_inputs();

        c.bench_function("benchmark", |b| {
            b.iter(|| {
                let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
                let result = indicator(&inputs, &OPTIONS_LIST, None).expect("EMV indicator failed");
                black_box(&result);
            });
        });
    }
}

/// Benchmark the Rust from_state implementation of EMV.
fn bench_rust_emv_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("emv");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data = min_data(&OPTIONS_LIST);
                    // First chunk
                    let chunk_inputs = [&high[..min_data], &low[..min_data], &volume[..min_data]];

                    let (_, mut state) = indicator(&chunk_inputs, &OPTIONS_LIST, None)
                        .expect("EMV indicator failed");

                    // Chunks
                    let mut high_chunks = high[min_data..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume[min_data..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), volume_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(volume_chunks.by_ref())
                    {
                        let chunk_inputs = [high_chunk, low_chunk, volume_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, volume_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "emv",
                "Rust_FromState",
                &OPTIONS_LIST,
                n,
                &timing,
                Some(&stock_symbol),
            );

            // --- Rust_FromState_1_Bar benchmark ---
            if inputs[0].len() > 1 {
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
                    indicator(&new_inputs, &OPTIONS_LIST, None).expect("Rust EMV indicator failed");

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust EMV from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "emv",
                    "Rust_FromState_1_Bar",
                    &OPTIONS_LIST,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar_json benchmark ---
                let (_, state) =
                    indicator(&new_inputs, &OPTIONS_LIST, None).expect("Rust EMV indicator failed");
                let json = serde_json::to_string(&state).expect("json failed");
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut state: IndicatorState =
                            serde_json::from_str(&json).expect("JSON failed");
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust EMV from state indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "emv",
                    "Rust_FromState_1_Bar_json",
                    &OPTIONS_LIST,
                    n,
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low, volume) = expand_inputs();

        let min_data = min_data(&OPTIONS_LIST);
        // First chunk
        let chunk_inputs = [&high[..min_data], &low[..min_data], &volume[..min_data]];

        let (_, mut state) =
            indicator(&chunk_inputs, &OPTIONS_LIST, None).expect("EMV indicator failed");

        c.bench_function("benchmark", |b| {
            b.iter(|| {
                let mut high_chunks = high[min_data..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[min_data..].chunks_exact(CHUNK_SIZE);

                for ((high_chunk, low_chunk), volume_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let chunk_inputs = [high_chunk, low_chunk, volume_chunk];
                    let result = state.batch_indicator(&chunk_inputs, None);
                    black_box(&result);
                }

                // Remainder
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let volume_rem = volume_chunks.remainder();

                if !high_rem.is_empty() {
                    let high_vec = high_rem.to_vec();
                    let low_vec = low_rem.to_vec();
                    let volume_vec = volume_rem.to_vec();
                    let _chunk_inputs = [&high_vec, &low_vec, &volume_vec];
                    let result = state.batch_indicator(&[&high_vec, &low_vec, &volume_vec], None);
                    black_box(&result);
                }
            });
        });
    }
}

/// Benchmark the Rust SIMD by assets implementation of EMV.
fn bench_rust_emv_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("emv");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                (
                    symbol.clone(),
                    data.iter().map(|d| d.high).collect(),
                    data.iter().map(|d| d.low).collect(),
                    data.iter().map(|d| d.volume).collect(),
                )
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // volume
            ],
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = indicator_by_assets::<4>(&inputs, &OPTIONS_LIST, None)
                    .expect("Rust SIMD by assets EMV indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "emv",
            "Rust_SIMD_by_assets",
            &OPTIONS_LIST,
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low, volume) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high, &low, &volume],
            &[&high, &low, &volume],
            &[&high, &low, &volume],
            &[&high, &low, &volume],
        ];

        c.bench_function("benchmark", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &OPTIONS_LIST, None)
                    .expect("Rust SIMD by assets EMV indicator failed");
                black_box(&result);
            });
        });
    }
}

/// Benchmark the Rust implementation of EMV with optional outputs.
fn bench_rust_emv_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("emv");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&inputs, &OPTIONS_LIST, Some(&[true]))
                        .expect("Rust EMV indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "emv",
                "Rust_optional",
                &OPTIONS_LIST,
                n,
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high, low, volume) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
        c.bench_function("Rust EMV", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &OPTIONS_LIST, Some(&[true]))
                    .expect("Rust EMV indicator failed");
                black_box(&result);
            });
        });
    }
}

criterion_group!(
    emv_benchmarks,
    bench_rust_emv_simd_by_assets,
    bench_rust_emv,
    bench_rust_emv_from_state,
    bench_c_emv,
    bench_rust_emv_optional
);
criterion_main!(emv_benchmarks);
