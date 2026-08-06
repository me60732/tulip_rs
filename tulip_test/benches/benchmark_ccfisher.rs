use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicator_types::TIndicatorState;
use tulip_rs::indicators::ccfisher::{
    CcFisher, Indicator, indicator_by_assets, indicator_by_options,
};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

const SAMPLE_SIZE: usize = 10000;
const CHUNK_SIZE: usize = 100;

/// Four α values to exercise across all benchmark variants.
const OPTIONS_LIST: [[f64; 1]; 4] = [[0.07], [0.10], [0.15], [0.0]];

const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

fn expand_inputs() -> Vec<f64> {
    let mut v = CLOSE.to_vec();
    for _ in 0..499 {
        v.extend_from_slice(&CLOSE);
    }
    v // ~7500 bars
}

fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Full-run scalar (fisher + signal only, no optionals)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ccfisher(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ccfisher");
        let data = get_all_stock_data().unwrap();
        for options in OPTIONS_LIST {
            for (stock_symbol, stock_data) in data {
                let close = get_close_array(stock_data);
                let n = close.len();
                let inputs = [close.as_slice()];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = CcFisher::indicator(&inputs, &options, None).expect("CCFisher failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ccfisher", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("ccfisher_rust");
        group.sample_size(SAMPLE_SIZE);
        for options in OPTIONS_LIST {
            group.bench_function(format!("Rust CCFisher (alpha={})", options[0]), |b| {
                b.iter(|| {
                    let result = CcFisher::indicator(&inputs, &options, None).expect("CCFisher failed");
                    black_box(&result);
                });
            });
        }
        group.finish();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Full-run scalar with TrendMode optional output
// Benchmarks the extra cost of computing TrendMode alongside Fisher/Signal.
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ccfisher_with_trendmode(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ccfisher");
        let data = get_all_stock_data().unwrap();
        for options in OPTIONS_LIST {
            for (stock_symbol, stock_data) in data {
                let close = get_close_array(stock_data);
                let n = close.len();
                let inputs = [close.as_slice()];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = CcFisher::indicator(&inputs, &options, Some(&[true, false, false]))
                            .expect("CCFisher+TrendMode failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ccfisher",
                    "Rust_optional",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("ccfisher_rust_with_trendmode");
        group.sample_size(SAMPLE_SIZE);
        for options in OPTIONS_LIST {
            group.bench_function(
                format!("Rust CCFisher+TrendMode (alpha={})", options[0]),
                |b| {
                    b.iter(|| {
                        let result = CcFisher::indicator(&inputs, &options, Some(&[true, false, false]))
                            .expect("CCFisher+TrendMode failed");
                        black_box(&result);
                    });
                },
            );
        }
        group.finish();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Streaming from saved state: chunked update + single-bar update
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ccfisher_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ccfisher");
        let data = get_all_stock_data().unwrap();
        for options in OPTIONS_LIST {
            for (stock_symbol, stock_data) in data {
                let close = get_close_array(stock_data);
                let n = close.len();

                // Chunked from-state
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let seed = CcFisher::min_data(&options).max(CHUNK_SIZE);
                        let (_, mut state) = CcFisher::indicator(&[&close[..seed]], &options, None)
                            .expect("CCFisher seed failed");
                        for chunk in close[seed..].chunks_exact(CHUNK_SIZE) {
                            black_box(
                                state
                                    .batch_indicator(&[chunk], None)
                                    .expect("batch_indicator failed"),
                            );
                        }
                        let rem = &close[seed + (close[seed..].len() / CHUNK_SIZE) * CHUNK_SIZE..];
                        if !rem.is_empty() {
                            black_box(
                                state
                                    .batch_indicator(&[rem], None)
                                    .expect("batch_indicator failed"),
                            );
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ccfisher",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // Single-bar update
                if n > 1 {
                    let (_, mut state) = CcFisher::indicator(&[&close[..n - 1]], &options, None)
                        .expect("CCFisher seed (1-bar) failed");
                    let final_input = [&close[n - 1..]];
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            black_box(
                                state
                                    .batch_indicator(&final_input, None)
                                    .expect("CCFisher 1-bar update failed"),
                            );
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "ccfisher",
                        "Rust_FromState_1_Bar",
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
            let seed = CcFisher::min_data(&options).max(CHUNK_SIZE);
            let (_, mut state) =
                CcFisher::indicator(&[&close_vec[..seed]], &options, None).expect("CCFisher seed failed");

            let mut group = c.benchmark_group("ccfisher_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust CCFisher from state (alpha={})", options[0]),
                |b| {
                    b.iter(|| {
                        for chunk in close_vec[seed..].chunks_exact(CHUNK_SIZE) {
                            black_box(
                                state
                                    .batch_indicator(&[chunk], None)
                                    .expect("batch_indicator failed"),
                            );
                        }
                    });
                },
            );
            group.finish();
        }

        for options in OPTIONS_LIST {
            if close_vec.len() > 1 {
                let (_, mut state) =
                    CcFisher::indicator(&[&close_vec[..close_vec.len() - 1]], &options, None)
                        .expect("CCFisher seed (1-bar) failed");
                let final_input = [&close_vec[close_vec.len() - 1..]];
                let mut group = c.benchmark_group("ccfisher_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!("Rust CCFisher from state 1 bar (alpha={})", options[0]),
                    |b| {
                        b.iter(|| {
                            black_box(
                                state
                                    .batch_indicator(&final_input, None)
                                    .expect("CCFisher 1-bar update failed"),
                            );
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMD by_assets: 4 assets processed simultaneously
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ccfisher_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ccfisher");
        let data = get_all_stock_data().unwrap();
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
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
                        .expect("SIMD by_assets CCFisher failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "ccfisher",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let close_vec = expand_inputs();
        let inputs: [&[&[f64]; 1]; 4] =
            [&[&close_vec], &[&close_vec], &[&close_vec], &[&close_vec]];
        let mut group = c.benchmark_group("ccfisher_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        for options in OPTIONS_LIST {
            group.bench_function(
                format!("Rust SIMD by_assets CCFisher (N=4, alpha={})", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by_assets CCFisher failed");
                        black_box(&result);
                    });
                },
            );
        }
        group.finish();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMD by_options: 4 α values simultaneously on one asset
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ccfisher_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ccfisher");
        let data = get_all_stock_data().unwrap();
        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("SIMD by_options CCFisher failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "ccfisher",
                "Rust_SIMD",
                &[0.0],
                n,
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
        let mut group = c.benchmark_group("ccfisher_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by_options CCFisher (4 alpha lanes)", |b| {
            b.iter(|| {
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by_options CCFisher failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_ccfisher_simd_by_assets,
    bench_ccfisher_simd_by_options,
    bench_ccfisher,
    bench_ccfisher_with_trendmode,
    bench_ccfisher_from_state,
);
criterion_main!(benches);
