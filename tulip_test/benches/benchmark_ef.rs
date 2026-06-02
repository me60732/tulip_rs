use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tulip_rs::indicators::ef::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Test data — same series used by the KAMA benchmark
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options from benchmark_kama.rs
const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

// Chunk size for batched processing
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..499 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

fn bench_rust_ef(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ef");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            indicator(&inputs, &options, None).expect("Rust EF indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ef", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!("Rust EF {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs = [close.as_slice()];
                    let result = indicator(&inputs, &options, None).expect("EF indicator failed");
                    black_box(&result);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_ef_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ef");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data = min_data(&options);
                        let chunk_inputs = [&close[..min_data]];

                        let (_, mut state) =
                            indicator(&chunk_inputs, &options, None).expect("EF indicator failed");

                        let mut close_chunks = close[min_data..].chunks_exact(CHUNK_SIZE);
                        for close_chunk in close_chunks.by_ref() {
                            let chunk_inputs = [close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        let close_rem = close_chunks.remainder();
                        if !close_rem.is_empty() {
                            let chunk_inputs = [close_rem];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ef",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let new_inputs = [&close[..close.len() - 1]];
                    let final_inputs = [&close[close.len() - 1..]];
                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust EF indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust EF from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "ef",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust EF indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust EF from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "ef",
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
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let min_data = min_data(&options);
            let chunk_inputs = [&close_vec[..min_data]];

            let (_, mut state) =
                indicator(&chunk_inputs, &options, None).expect("EF indicator failed");

            let mut group =
                c.benchmark_group(format!("Rust EF from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let mut close_chunks = close_vec[min_data..].chunks_exact(CHUNK_SIZE);
                    for close_chunk in close_chunks.by_ref() {
                        let chunk_inputs = [close_chunk];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                    let close_rem = close_chunks.remainder();
                    if !close_rem.is_empty() {
                        let chunk_inputs = [close_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if close_vec.len() > 1 {
                let new_inputs = [&close_vec[..close_vec.len() - 1]];
                let final_inputs = [&close_vec[close_vec.len() - 1..]];
                let (_, mut state) =
                    indicator(&new_inputs, &options, None).expect("Rust EF indicator failed");

                let mut group =
                    c.benchmark_group(format!("Rust EF from state 1 bar {{ {:.1} }}", options[0]));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(&final_inputs, None)
                            .expect("Rust EF from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

fn bench_rust_ef_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ef");

        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
            .collect();

        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets EF indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "ef",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let close = expand_inputs();
        let inputs: [&[&[f64]; 1]; 4] = [&[&close], &[&close], &[&close], &[&close]];

        for options in OPTIONS_LIST {
            c.bench_function(&format!("SIMD by assets EF {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets EF indicator failed");
                    black_box(&result);
                });
            });
        }
    }
}

fn bench_rust_ef_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ef");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [close_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let options_4 = [
                        &OPTIONS_LIST[0],
                        &OPTIONS_LIST[1],
                        &OPTIONS_LIST[2],
                        &OPTIONS_LIST[3],
                    ];
                    let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD by options EF indicator failed");
                    black_box(&result_4);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ef",
                "Rust_SIMD",
                &[0.0],
                close_vec.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        let mut group = c.benchmark_group("ef_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options EF (4 lanes)", |b| {
            b.iter(|| {
                let result_4 = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD by options EF indicator failed");
                black_box(&result_4);
            });
        });
        group.finish();
    }
}

fn bench_rust_ta_ef(c: &mut Criterion) {
    use ta::indicators::EfficiencyRatio;
    use ta::Next;

    if should_log_to_db() {
        init_database_data();
        init_logging("ef");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();

            for options in OPTIONS_LIST {
                let period = options[0] as usize;
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let mut er =
                            EfficiencyRatio::new(period).expect("ta EfficiencyRatio new failed");
                        let mut last = 0.0_f64;
                        for &price in &close_vec {
                            last = er.next(price);
                        }
                        black_box(last);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "ef",
                    "RustTa",
                    &options,
                    close_vec.len(),
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let close_vec = expand_inputs();

        for options in OPTIONS_LIST {
            let period = options[0] as usize;
            let mut group = c.benchmark_group("ef_rust_ta");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(format!("RustTa EF {{ {:.1} }}", options[0]), |b| {
                b.iter(|| {
                    let mut er =
                        EfficiencyRatio::new(period).expect("ta EfficiencyRatio new failed");
                    let mut last = 0.0_f64;
                    for &price in &close_vec {
                        last = er.next(price);
                    }
                    black_box(last);
                });
            });
            group.finish();
        }
    }
}

criterion_group!(
    ef_benchmarks,
    bench_rust_ef_simd_by_assets,
    bench_rust_ef_simd_by_options,
    bench_rust_ef,
    bench_rust_ta_ef,
    bench_rust_ef_from_state,
);
criterion_main!(ef_benchmarks);
