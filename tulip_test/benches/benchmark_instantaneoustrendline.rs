use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicator_types::TIndicatorState;
use tulip_rs::indicators::instantaneoustrendline::{InstantaneousTrendline, Indicator, indicator_by_assets};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ht_trendline, ta_ht_trendline_start};

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
fn bench_it(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("instantaneoustrendline");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = InstantaneousTrendline::indicator(&inputs, &[], None).expect("IT failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "instantaneoustrendline",
                "Rust",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let mut group = c.benchmark_group("instantaneoustrendline_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust Instantaneous Trendline", |b| {
            b.iter(|| {
                let result = InstantaneousTrendline::indicator(&inputs, &[], None).expect("IT failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Streaming from saved state: chunked + single-bar variants.
fn bench_it_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("instantaneoustrendline");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            // --- chunked from-state ---
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let seed = InstantaneousTrendline::min_data(&[]).max(CHUNK_SIZE);
                    let (_, mut state) = InstantaneousTrendline::indicator(&[&close[..seed]], &[], None)
                        .expect("IT seed failed");
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
                "instantaneoustrendline",
                "Rust_FromState",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );

            // --- single-bar update ---
            if n > 1 {
                let (_, mut state) = InstantaneousTrendline::indicator(&[&close[..n - 1]], &[], None)
                    .expect("IT seed (1-bar) failed");
                let final_input = [&close[n - 1..]];
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        black_box(
                            state
                                .batch_indicator(&final_input, None)
                                .expect("IT 1-bar update failed"),
                        );
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "instantaneoustrendline",
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
        let seed = InstantaneousTrendline::min_data(&[]).max(CHUNK_SIZE);
        let (_, mut state) = InstantaneousTrendline::indicator(&[&close_vec[..seed]], &[], None)
            .expect("IT seed failed");

        let mut group = c.benchmark_group("instantaneoustrendline_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust IT from state", |b| {
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
            let (_, mut state) = InstantaneousTrendline::indicator(&[&close_vec[..close_vec.len() - 1]], &[], None)
                .expect("IT seed (1-bar) failed");
            let final_input = [&close_vec[close_vec.len() - 1..]];
            let mut group = c.benchmark_group("instantaneoustrendline_rust_from_state_1_bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("Rust IT from state 1 bar", |b| {
                b.iter(|| {
                    black_box(
                        state
                            .batch_indicator(&final_input, None)
                            .expect("IT 1-bar update failed"),
                    );
                });
            });
            group.finish();
        }
    }
}

/// SIMD by_assets: 4 assets processed simultaneously.
fn bench_it_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("instantaneoustrendline");
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
                    .expect("SIMD by_assets IT failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );
        log_timing_result(
            "instantaneoustrendline",
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
        let mut group = c.benchmark_group("instantaneoustrendline_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by_assets IT (N=4)", |b| {
            b.iter(|| {
                let result = indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("SIMD by_assets IT failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// TA-Lib HT_TRENDLINE — **throughput comparison only**.
///
/// TA-Lib's `HT_TRENDLINE` implements a variable-length SMA + 4-bar WMA, **not**
/// Ehlers' 2-pole IIR. This benchmark measures throughput only; the algorithms are
/// fundamentally different and the outputs cannot be compared for correctness.
#[cfg(feature = "talib")]
fn bench_talib_ht_trendline(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("instantaneoustrendline");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];
            let lookback = ta_ht_trendline_start();
            assert!(lookback >= 0);
            let out_len = n - lookback as usize;
            let mut out_trendline = vec![0.0_f64; out_len];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut outputs: Vec<*mut f64> = vec![out_trendline.as_mut_ptr()];
                    let ret = ta_ht_trendline(
                        n as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_ht_trendline returned error {ret}");
                    black_box(&out_trendline);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "instantaneoustrendline",
                "talib",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close = expand_inputs();
        let n = close.len();
        let inputs: Vec<*const f64> = vec![close.as_ptr()];
        let lookback = ta_ht_trendline_start();
        assert!(lookback >= 0);
        let out_len = n - lookback as usize;
        let mut group = c.benchmark_group("instantaneoustrendline_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib HT_TRENDLINE (different algorithm — throughput only)", |b| {
            b.iter(|| {
                let mut out_trendline = vec![0.0_f64; out_len];
                let mut outputs: Vec<*mut f64> = vec![out_trendline.as_mut_ptr()];
                let ret = ta_ht_trendline(
                    n as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_ht_trendline returned error {ret}");
                black_box(&out_trendline);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_it_simd_by_assets,
    bench_it,
    bench_talib_ht_trendline,
    bench_it_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_it_simd_by_assets,
    bench_it,
    bench_it_from_state,
);
criterion_main!(benches);
