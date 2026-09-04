use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::ichimoku::{Ichimoku, Indicator, IndicatorByOptions, TIndicatorState};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

//const SAMPLE_SIZE: usize = 10000;
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

// [short_period, long_period] — standard Ichimoku settings and an alternative
const OPTIONS_LIST: [[f64; 2]; 4] = [[9.0, 26.0], [7.0, 22.0], [5.0, 15.0], [14.0, 45.0]];

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
    (high_vec, low_vec, close_vec) // ~7515 bars
}

fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

/// Benchmark the full Ichimoku indicator (no optional outputs).
fn bench_rust_ichimoku(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ichimoku");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = Ichimoku::indicator(&inputs, &options, None)
                            .expect("Rust Ichimoku indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("ichimoku", "Rust", &options, n, &timing, Some(stock_symbol));
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
            let mut group = c.benchmark_group("ichimoku_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust Ichimoku {{ short: {}, long: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = Ichimoku::indicator(&inputs, &options, None)
                            .expect("Rust Ichimoku indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark Ichimoku with the optional `lagging_span` output enabled.
///
/// `lagging_span` is a verbatim copy of the close input (`close.to_vec()`), so this
/// benchmark quantifies the allocation cost of that optional vec.
fn bench_rust_ichimoku_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ichimoku");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = Ichimoku::indicator(&inputs, &options, Some(&[true]))
                            .expect("Rust Ichimoku optional indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ichimoku",
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
            let mut group = c.benchmark_group("ichimoku_rust_optional");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust Ichimoku Optional {{ short: {}, long: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = Ichimoku::indicator(&inputs, &options, Some(&[true]))
                            .expect("Rust Ichimoku optional indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark Ichimoku using `batch_indicator` for streaming updates.
///
/// Seeds the state with `min_data` bars then processes the remainder in
/// `CHUNK_SIZE`-bar chunks — matching the real-time usage pattern.
/// Also measures the single-bar update cost separately.
fn bench_rust_ichimoku_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ichimoku");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                // --- chunked from-state ---
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let seed = Ichimoku::min_data(&options).max(CHUNK_SIZE);
                        let (_, mut state) = Ichimoku::indicator(
                            &[&high[..seed], &low[..seed], &close[..seed]],
                            &options,
                            None,
                        )
                        .expect("Ichimoku seed failed");

                        let mut hc = high[seed..].chunks_exact(CHUNK_SIZE);
                        let mut lc = low[seed..].chunks_exact(CHUNK_SIZE);
                        let mut cc = close[seed..].chunks_exact(CHUNK_SIZE);
                        for ((h, l), c) in hc.by_ref().zip(lc.by_ref()).zip(cc.by_ref()) {
                            black_box(
                                state
                                    .batch_indicator(&[h, l, c], None)
                                    .expect("Ichimoku batch_indicator failed"),
                            );
                        }
                        let (hr, lr, cr) = (hc.remainder(), lc.remainder(), cc.remainder());
                        if !hr.is_empty() {
                            black_box(
                                state
                                    .batch_indicator(&[hr, lr, cr], None)
                                    .expect("Ichimoku batch_indicator failed"),
                            );
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "ichimoku",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- single-bar update ---
                if n > 1 {
                    let (_, mut state) = Ichimoku::indicator(
                        &[&high[..n - 1], &low[..n - 1], &close[..n - 1]],
                        &options,
                        None,
                    )
                    .expect("Ichimoku seed (1-bar) failed");

                    let final_inputs = [&high[n - 1..], &low[n - 1..], &close[n - 1..]];
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            black_box(
                                state
                                    .batch_indicator(&final_inputs, None)
                                    .expect("Ichimoku 1-bar update failed"),
                            );
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "ichimoku",
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
        let (high_vec, low_vec, close_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let seed = Ichimoku::min_data(&options).max(CHUNK_SIZE);

            let mut group = c.benchmark_group("ichimoku_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust Ichimoku from state {{ short: {}, long: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let (_, mut state) = Ichimoku::indicator(
                            &[&high_vec[..seed], &low_vec[..seed], &close_vec[..seed]],
                            &options,
                            None,
                        )
                        .expect("Ichimoku seed failed");

                        let mut hc = high_vec[seed..].chunks_exact(CHUNK_SIZE);
                        let mut lc = low_vec[seed..].chunks_exact(CHUNK_SIZE);
                        let mut cc = close_vec[seed..].chunks_exact(CHUNK_SIZE);
                        for ((h, l), c) in hc.by_ref().zip(lc.by_ref()).zip(cc.by_ref()) {
                            black_box(
                                state
                                    .batch_indicator(&[h, l, c], None)
                                    .expect("Ichimoku batch_indicator failed"),
                            );
                        }
                        let (hr, lr, cr) = (hc.remainder(), lc.remainder(), cc.remainder());
                        if !hr.is_empty() {
                            black_box(
                                state
                                    .batch_indicator(&[hr, lr, cr], None)
                                    .expect("Ichimoku batch_indicator failed"),
                            );
                        }
                    });
                },
            );
            group.finish();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMD by_assets: 4 assets processed simultaneously
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ichimoku_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ichimoku");
        let data = get_all_stock_data().unwrap();
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| {
                let (high, low, close) = get_arrays(eod);
                (sym.clone(), high, low, close)
            })
            .collect();
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3],
            &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3],
            &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3],
            &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3],
        ];
        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = Ichimoku::indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("SIMD by_assets Ichimoku failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "ichimoku",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let (high_vec, low_vec, close_vec) = expand_inputs();
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
            &[&high_vec, &low_vec, &close_vec],
        ];
        let mut group = c.benchmark_group("ichimoku_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        for options in OPTIONS_LIST {
            group.bench_function(
                format!(
                    "Rust SIMD by_assets Ichimoku (N=4, short={}, long={})",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = Ichimoku::indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by_assets Ichimoku failed");
                        black_box(&result);
                    });
                },
            );
        }
        group.finish();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SIMD by_options: 4 option sets simultaneously on one asset
// ─────────────────────────────────────────────────────────────────────────────

fn bench_ichimoku_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ichimoku");
        let data = get_all_stock_data().unwrap();
        let options_4: [&[f64; 2]; 4] = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = Ichimoku::indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("SIMD by_options Ichimoku failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "ichimoku",
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
        let options_4: [&[f64; 2]; 4] = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];
        let mut group = c.benchmark_group("ichimoku_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by_options Ichimoku (4 period-set lanes)", |b| {
            b.iter(|| {
                let result = Ichimoku::indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by_options Ichimoku failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_ichimoku_simd_by_assets,
    bench_ichimoku_simd_by_options,
    bench_rust_ichimoku,
    bench_rust_ichimoku_optional,
    bench_rust_ichimoku_from_state,
);
criterion_main!(benches);
