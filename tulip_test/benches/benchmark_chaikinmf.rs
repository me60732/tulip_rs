use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::chaikinmf::{
    ChaikinMf, Indicator, indicator_by_assets, indicator_by_options, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data (high, low, close, volume)
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

const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

/// Chunk size for from-state benchmarks.
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it.
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

// Helper to extract HLCV arrays from stock data.
fn get_hlcv_arrays(
    stock_data: &[tulip_test::database::EodData],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
    (high, low, close, volume)
}

/// Benchmark the Rust Chaikin Money Flow ChaikinMf::indicator (full one-shot).
fn bench_rust_chaikinmf(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("chaikinmf");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let n = high.len();
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = ChaikinMf::indicator(&inputs, &options, None)
                            .expect("Chaikin MF ChaikinMf::indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "chaikinmf",
                    "Rust",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
            volume_vec.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("chaikinmf_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("Rust Chaikin MF {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result =
                        ChaikinMf::indicator(&inputs, &options, None).expect("Chaikin MF ChaikinMf::indicator failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust Chaikin Money Flow ChaikinMf::indicator using stateful chunked processing.
fn bench_rust_chaikinmf_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("chaikinmf");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                // --- Rust_FromState (chunked) ---
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = ChaikinMf::min_data(&options).max(CHUNK_SIZE);

                        let chunk_inputs = [
                            &high[..min_data_val],
                            &low[..min_data_val],
                            &close[..min_data_val],
                            &volume[..min_data_val],
                        ];
                        let (_, mut state) = ChaikinMf::indicator(&chunk_inputs, &options, None)
                            .expect("Chaikin MF ChaikinMf::indicator failed");

                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (((hc, lc), cc), vc) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                            .zip(volume_chunks.by_ref())
                        {
                            let chunk_inputs = [hc, lc, cc, vc];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

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
                    "chaikinmf",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar ---
                if n > 1 {
                    let new_inputs = [
                        &high[..n - 1],
                        &low[..n - 1],
                        &close[..n - 1],
                        &volume[..n - 1],
                    ];
                    let final_inputs = [
                        &high[n - 1..],
                        &low[n - 1..],
                        &close[n - 1..],
                        &volume[n - 1..],
                    ];
                    let (_, mut state) = ChaikinMf::indicator(&new_inputs, &options, None)
                        .expect("Chaikin MF ChaikinMf::indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Chaikin MF from-state (1 bar) failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "chaikinmf",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json ---
                    let (_, state) = ChaikinMf::indicator(&new_inputs, &options, None)
                        .expect("Chaikin MF ChaikinMf::indicator failed");
                    let json = serde_json::to_string(&state).expect("JSON serialization failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON deserialization failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Chaikin MF from-state (1 bar, JSON) failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "chaikinmf",
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
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!(
                "Rust Chaikin MF from state {{ {:.1} }}",
                options[0]
            ));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = ChaikinMf::min_data(&options).max(CHUNK_SIZE);

                    let chunk_inputs = [
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                        &volume_vec[..min_data_val],
                    ];
                    let (_, mut state) = ChaikinMf::indicator(&chunk_inputs, &options, None)
                        .expect("Chaikin MF ChaikinMf::indicator failed");

                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (((hc, lc), cc), vc) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                        .zip(volume_chunks.by_ref())
                    {
                        let chunk_inputs = [hc, lc, cc, vc];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }

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
            group.finish();

            // 1-bar from-state
            let n = high_vec.len();
            if n > 1 {
                let new_inputs = [
                    &high_vec[..n - 1],
                    &low_vec[..n - 1],
                    &close_vec[..n - 1],
                    &volume_vec[..n - 1],
                ];
                let final_inputs = [
                    &high_vec[n - 1..],
                    &low_vec[n - 1..],
                    &close_vec[n - 1..],
                    &volume_vec[n - 1..],
                ];
                let (_, mut state) =
                    ChaikinMf::indicator(&new_inputs, &options, None).expect("Chaikin MF ChaikinMf::indicator failed");

                let mut group = c.benchmark_group(format!(
                    "Rust Chaikin MF from state 1 bar {{ {:.1} }}",
                    options[0]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Chaikin MF from-state (1 bar) failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

/// Benchmark SIMD by-assets: 4 assets computed simultaneously.
fn bench_chaikinmf_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("chaikinmf");
        let data = get_all_stock_data().unwrap();
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close, volume) = get_hlcv_arrays(eod);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();
        let n = stock_data[0].1.len();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 4] = [
                &stock_data[0].1,
                &stock_data[0].2,
                &stock_data[0].3,
                &stock_data[0].4,
            ];
            let asset1: [&[f64]; 4] = [
                &stock_data[1].1,
                &stock_data[1].2,
                &stock_data[1].3,
                &stock_data[1].4,
            ];
            let asset2: [&[f64]; 4] = [
                &stock_data[2].1,
                &stock_data[2].2,
                &stock_data[2].3,
                &stock_data[2].4,
            ];
            let asset3: [&[f64]; 4] = [
                &stock_data[3].1,
                &stock_data[3].2,
                &stock_data[3].3,
                &stock_data[3].4,
            ];
            let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs_4, &options, None)
                        .expect("SIMD by-assets ChaikinMF failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "chaikinmf",
                "Rust_SIMD_by_assets",
                &options,
                n,
                &timing,
                None,
            );
        }
    } else {
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let asset: [&[f64]; 4] = [&high_vec, &low_vec, &close_vec, &volume_vec];
        let inputs_4: [&[&[f64]; 4]; 4] = [&asset, &asset, &asset, &asset];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("chaikinmf_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("SIMD ByAssets {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator_by_assets::<4>(&inputs_4, &options, None)
                        .expect("SIMD by-assets ChaikinMF failed");
                    black_box(&result);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark SIMD by-options: 4 periods computed simultaneously on one asset.
fn bench_chaikinmf_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];

    if should_log_to_db() {
        init_database_data();
        init_logging("chaikinmf");
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let n = high.len();
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("SIMD by-options ChaikinMF failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "chaikinmf",
                "Rust_SIMD",
                &[OPTIONS_LIST[0][0]],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec, volume_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
            volume_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("chaikinmf_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("SIMD ByOptions {5/10/14/20}", |b| {
            b.iter(|| {
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by-options ChaikinMF failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_chaikinmf_simd_by_assets,
    bench_chaikinmf_simd_by_options,
    bench_rust_chaikinmf,
    bench_rust_chaikinmf_from_state,
);
criterion_main!(benches);
