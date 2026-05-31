use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::tr::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_tr, ti_tr_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_tr, ta_tr_start};

// Sample input data (high, low, close prices)
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
    87.32, 87.29,
];

// Options for TR (no options)
const OPTIONS: [f64; 0] = [];

// Chunk size for from_state benchmarks
const CHUNK_SIZE: usize = 100;

use tulip_rs::indicators::tr::indicator_by_assets as rust_tr_simd;

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

/// Benchmark the C implementation of TR.
fn bench_c_tr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs: Vec<*const f64> =
                vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_tr_start(OPTIONS.as_ptr()) };
                    assert!(start_index >= 0, "ti_tr_start returned a negative index");
                    let output_len = high_vec.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_tr(
                            high_vec.len() as i32,
                            inputs.as_ptr(),
                            OPTIONS.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_tr returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "tr",
                "C_tulip",
                &OPTIONS,
                high_vec.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        let start_index = unsafe { ti_tr_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_tr_start returned a negative index");
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("tr_c");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C TR", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_tr(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        OPTIONS.as_ptr(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_tr returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust implementation of TR.
fn bench_rust_tr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&inputs, &OPTIONS, None); //.expect("Rust TR indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "tr",
                "Rust",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("tr_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust TR", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &OPTIONS, None).expect("Rust TR indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust from_state implementation of TR.
fn bench_rust_tr_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
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
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &OPTIONS, None).expect("Rust TR indicator failed");

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
                "tr",
                "Rust_FromState",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(stock_symbol),
            );

            let (_, mut state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust TR indicator failed");

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust TR from state indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "tr",
                "Rust_FromState_1_Bar",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(stock_symbol),
            );

            let (_, state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust TR indicator failed");
            let json = serde_json::to_string(&state).expect("json failed");

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut state: IndicatorState =
                        serde_json::from_str(&json).expect("JSON failed");
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust TR from state indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "tr",
                "Rust_FromState_1_Bar_json",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();

        let mut group = c.benchmark_group("tr_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust TR from state", |b| {
            b.iter(|| {
                let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
                // First chunk
                let chunk_inputs = [
                    &high_vec[..min_data_val],
                    &low_vec[..min_data_val],
                    &close_vec[..min_data_val],
                ];

                let (_, mut state) =
                    indicator(&chunk_inputs, &OPTIONS, None).expect("Rust TR indicator failed");

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
    }
}

/// Benchmark the TA-Lib implementation of TR.
#[cfg(feature = "talib")]
fn bench_talib_tr(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let n = high_vec.len();
            let inputs: Vec<*const f64> =
                vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

            let mut timing = TimingMeasurements::new();

            timing.measure(
                || {
                    let start_index = ta_tr_start();
                    assert!(start_index >= 0, "ta_tr_start returned a negative index");
                    let output_len = high_vec.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                    let ret = ta_tr(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_tr returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("tr", "talib", &OPTIONS, n, &timing, Some(stock_symbol));
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr(), close_vec.as_ptr()];

        let start_index = ta_tr_start();
        assert!(start_index >= 0, "ta_tr_start returned a negative index");
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("tr_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib TR", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = ta_tr(
                    high_vec.len() as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_tr returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust SIMD by assets implementation of TR.
fn bench_rust_tr_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

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
                    data.iter().map(|d| d.close).collect(),
                )
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // close
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // close
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // close
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // close
            ],
        ];

        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = rust_tr_simd::<4>(&inputs, &OPTIONS, None)
                    .expect("Rust SIMD by assets TR indicator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );

        log_timing_result(
            "tr",
            "Rust_SIMD_by_assets",
            &OPTIONS,
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec, close_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
        ];

        let mut group = c.benchmark_group("tr_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by assets TR", |b| {
            b.iter(|| {
                let result = rust_tr_simd::<4>(&inputs, &OPTIONS, None)
                    .expect("Rust SIMD by assets TR indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

fn bench_rust_ta_tr(c: &mut Criterion) {
    use ta::indicators::TrueRange;
    use ta::{DataItem, Next};

    if should_log_to_db() {
        init_database_data();
        init_logging("tr");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let open: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            
            let n = close.len();

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut tr = TrueRange::new();
                    let mut last = 0.0_f64;
                    for i in 0..high.len() {
                        let item = unsafe { DataItem::builder()
                            .high(*high.get_unchecked(i))
                            .low(*low.get_unchecked(i))
                            .close(*close.get_unchecked(i))
                            .open(*open.get_unchecked(i))
                            .volume(1000.0)
                            .build()
                            .expect("DataItem build failed")
                        };
                        last = tr.next(&item);
                    }
                    black_box(last);
                },
                SAMPLE_SIZE,
            );

            log_timing_result("tr", "RustTa", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();

        let mut group = c.benchmark_group("tr_rust_ta");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("RustTa TR", |b| {
            b.iter(|| {
                let mut tr = TrueRange::new();
                let mut last = 0.0_f64;
                for i in 0..high_vec.len() {
                    let h = high_vec[i].max(close_vec[i]);
                    let l = low_vec[i].min(close_vec[i]);
                    let item = DataItem::builder()
                        .high(h)
                        .low(l)
                        .close(close_vec[i])
                        .open(close_vec[i])
                        .volume(1000.0)
                        .build()
                        .expect("DataItem build failed");
                    last = tr.next(&item);
                }
                black_box(last);
            });
        });
        group.finish();
    }
}

//#[cfg(feature = "nightly")]
#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_tr_simd_by_assets,
    bench_rust_tr,
    bench_rust_ta_tr,
    bench_c_tr,
    bench_talib_tr,
    bench_rust_tr_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_tr_simd_by_assets,
    bench_rust_tr,
    bench_c_tr,
    bench_rust_tr_from_state,
    bench_rust_ta_tr,
);

criterion_main!(benches);
