use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicator_types::TIndicatorState;
use tulip_rs::indicators::adaptivemsw::{indicator, indicator_by_assets, min_data};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

const SAMPLE_SIZE: usize = 10000;
const CHUNK_SIZE: usize = 100;

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

/// Full run over all stock data bars.
fn bench_adaptivemsw(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("adaptivemsw");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&inputs, &[], None).expect("Adaptive MSW failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result("adaptivemsw", "Rust", &[], n, &timing, Some(stock_symbol));
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("adaptivemsw_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust Adaptive MESA Sine Wave", |b| {
            b.iter(|| {
                let result = indicator(&inputs, &[], None).expect("Adaptive MSW failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Streaming from saved state: chunked + single-bar variants.
fn bench_adaptivemsw_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("adaptivemsw");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let seed = min_data(&[]).max(CHUNK_SIZE);
                    let (_, mut state) =
                        indicator(&[&close[..seed]], &[], None).expect("AMSW seed failed");
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
                "adaptivemsw",
                "Rust_FromState",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );

            if n > 1 {
                let (_, mut state) =
                    indicator(&[&close[..n - 1]], &[], None).expect("AMSW seed (1-bar) failed");
                let final_input = [&close[n - 1..]];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        black_box(
                            state
                                .batch_indicator(&final_input, None)
                                .expect("AMSW 1-bar update failed"),
                        );
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "adaptivemsw",
                    "Rust_FromState_1_Bar",
                    &[],
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let close_vec = expand_inputs();
        let seed = min_data(&[]).max(CHUNK_SIZE);
        let (_, mut state) = indicator(&[&close_vec[..seed]], &[], None).expect("AMSW seed failed");

        let mut group = c.benchmark_group("adaptivemsw_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust Adaptive MSW from state", |b| {
            b.iter(|| {
                for chunk in close_vec[seed..].chunks_exact(CHUNK_SIZE) {
                    black_box(
                        state
                            .batch_indicator(&[chunk], None)
                            .expect("batch_indicator failed"),
                    );
                }
            });
        });
        group.finish();

        if close_vec.len() > 1 {
            let (_, mut state) = indicator(&[&close_vec[..close_vec.len() - 1]], &[], None)
                .expect("AMSW seed (1-bar) failed");
            let final_input = [&close_vec[close_vec.len() - 1..]];
            let mut group = c.benchmark_group("adaptivemsw_rust_from_state_1_bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("Rust Adaptive MSW from state 1 bar", |b| {
                b.iter(|| {
                    black_box(
                        state
                            .batch_indicator(&final_input, None)
                            .expect("AMSW 1-bar update failed"),
                    );
                });
            });
            group.finish();
        }
    }
}

/// SIMD by_assets: 4 assets processed simultaneously.
/// The HD runs in 4-lane SIMD; each asset's DFT uses 8-wide SIMD internally.
fn bench_adaptivemsw_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("adaptivemsw");
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
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("SIMD by_assets AMSW failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );
        log_timing_result(
            "adaptivemsw",
            "Rust_SIMD_by_assets",
            &[],
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        let close_vec = expand_inputs();
        let inputs: [&[&[f64]; 1]; 4] =
            [&[&close_vec], &[&close_vec], &[&close_vec], &[&close_vec]];
        let mut group = c.benchmark_group("adaptivemsw_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by_assets Adaptive MSW (N=4)", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("SIMD by_assets AMSW failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_adaptivemsw_simd_by_assets,
    bench_adaptivemsw,
    bench_adaptivemsw_from_state,
);
criterion_main!(benches);
