use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::supertrend::{SuperTrend, Indicator, TIndicatorState, IndicatorState, indicator_by_assets, indicator_by_options};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

//const SAMPLE_SIZE: usize = 30000;

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

// Options: [period, step]
const OPTIONS_LIST: [[f64; 2]; 4] = [[7.0, 3.0], [10.0, 3.0], [14.0, 2.0], [20.0, 2.0]];

const CHUNK_SIZE: usize = 100;

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

fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

fn bench_rust_supertrend(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = SuperTrend::indicator(&inputs, &options, None)
                            .expect("Rust Supertrend indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "supertrend",
                    "Rust",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
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
            let mut group = c.benchmark_group("supertrend_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust Supertrend {{ {}, {} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let result = SuperTrend::indicator(&inputs, &options, None)
                            .expect("Rust Supertrend indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_supertrend_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = SuperTrend::min_data(&options).max(CHUNK_SIZE);
                        let chunk_inputs = [
                            &high[..min_data_val],
                            &low[..min_data_val],
                            &close[..min_data_val],
                        ];
                        let (_, mut state) = SuperTrend::indicator(&chunk_inputs, &options, None)
                            .expect("Rust Supertrend indicator failed");

                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for ((hc, lc), cc) in high_chunks
                            .by_ref()
                            .zip(low_chunks.by_ref())
                            .zip(close_chunks.by_ref())
                        {
                            let result = state.batch_indicator(&[hc, lc, cc], None);
                            black_box(&result);
                        }

                        let hr = high_chunks.remainder();
                        let lr = low_chunks.remainder();
                        let cr = close_chunks.remainder();
                        if !hr.is_empty() {
                            let result = state.batch_indicator(&[hr, lr, cr], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "supertrend",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar ---
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
                    let (_, mut state) = SuperTrend::indicator(&new_inputs, &options, None)
                        .expect("Rust Supertrend indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust Supertrend from state failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "supertrend",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json ---
                    let (_, state) = SuperTrend::indicator(&new_inputs, &options, None)
                        .expect("Rust Supertrend indicator failed");
                    let json = serde_json::to_string(&state).expect("json serialization failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("json deserialization failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust Supertrend from state failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "supertrend",
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
                "Rust Supertrend from state {{ {:.1}, {:.1} }}",
                options[0], options[1]
            ));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let min_data_val = SuperTrend::min_data(&options).max(CHUNK_SIZE);
                    let chunk_inputs = [
                        &high_vec[..min_data_val],
                        &low_vec[..min_data_val],
                        &close_vec[..min_data_val],
                    ];
                    let (_, mut state) = SuperTrend::indicator(&chunk_inputs, &options, None)
                        .expect("Rust Supertrend indicator failed");

                    let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((hc, lc), cc) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let result = state.batch_indicator(&[hc, lc, cc], None);
                        black_box(&result);
                    }

                    let hr = high_chunks.remainder();
                    let lr = low_chunks.remainder();
                    let cr = close_chunks.remainder();
                    if !hr.is_empty() {
                        let result = state.batch_indicator(&[hr, lr, cr], None);
                        black_box(&result);
                    }
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_supertrend_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = SuperTrend::indicator(&inputs, &options, Some(&[true, true, true]))
                            .expect("Rust Supertrend optional indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "supertrend",
                    "Rust_optional",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
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
            let mut group = c.benchmark_group("supertrend_rust_optional");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust Supertrend Optional {{ {}, {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = SuperTrend::indicator(&inputs, &options, Some(&[true, true, true]))
                            .expect("Rust Supertrend optional indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_supertrend_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
        let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
        let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
        let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
        let inputs: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets Supertrend failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "supertrend",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();

        let asset: [&[f64]; 3] = [&high_vec, &low_vec, &close_vec];
        let inputs: [&[&[f64]; 3]; 4] = [&asset, &asset, &asset, &asset];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("supertrend_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust SIMD by assets Supertrend {{ {}, {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets Supertrend failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

fn bench_rust_supertrend_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let n = high.len();
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD by options Supertrend failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "supertrend",
                "Rust_SIMD",
                &[0.0, 0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("supertrend_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options Supertrend (4 lanes)", |b| {
            b.iter(|| {
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD by options Supertrend failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

fn bench_kand_supertrend(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("supertrend");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let period = options[0] as usize;
                let multiplier = options[1] as f64;
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut out_trend = vec![0_i64; n];
                        let mut out_supertrend = vec![0.0_f64; n];
                        let mut out_atr = vec![0.0_f64; n];
                        let mut out_upper = vec![0.0_f64; n];
                        let mut out_lower = vec![0.0_f64; n];
                        kand::ohlcv::supertrend::supertrend(
                            &high,
                            &low,
                            &close,
                            period,
                            multiplier,
                            &mut out_trend,
                            &mut out_supertrend,
                            &mut out_atr,
                            &mut out_upper,
                            &mut out_lower,
                        )
                        .expect("kand Supertrend failed");
                        black_box(&out_supertrend);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "supertrend",
                    "RustKanda",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let n = high_vec.len();

        for options in OPTIONS_LIST {
            let period = options[0] as usize;
            let multiplier = options[1] as f64;
            let mut group = c.benchmark_group("supertrend_kand");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("kand Supertrend {{ {}/{} }}", options[0], options[1]),
                |b| {
                    b.iter(|| {
                        let mut out_trend = vec![0i64; n];
                        let mut out_supertrend = vec![0.0f64; n];
                        let mut out_atr = vec![0.0f64; n];
                        let mut out_upper = vec![0.0f64; n];
                        let mut out_lower = vec![0.0f64; n];
                        kand::ohlcv::supertrend::supertrend(
                            &high_vec,
                            &low_vec,
                            &close_vec,
                            period,
                            multiplier,
                            &mut out_trend,
                            &mut out_supertrend,
                            &mut out_atr,
                            &mut out_upper,
                            &mut out_lower,
                        )
                        .expect("kand Supertrend failed");
                        black_box(&out_supertrend);
                    });
                },
            );
            group.finish();
        }
    }
}

criterion_group!(
    benches,
    bench_rust_supertrend_simd_by_assets,
    bench_rust_supertrend_simd_by_options,
    bench_rust_supertrend,
    bench_rust_supertrend_optional,
    bench_rust_supertrend_from_state,
    bench_kand_supertrend,
);
criterion_main!(benches);
