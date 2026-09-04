use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicator_types::TIndicatorState;
use tulip_rs::indicators::homodynediscriminator::{HomodyneDiscriminator, Indicator};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ht_dcperiod, ta_ht_dcperiod_start};
const SAMPLE_SIZE: usize = 10000;
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..499 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec // ~7500 bars
}

fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

/// Benchmark the full Homodyne Discriminator over ~7500 bars.
fn bench_homodynediscriminator(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("homodynediscriminator");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = HomodyneDiscriminator::indicator(&inputs, &[], None)
                        .expect("Homodyne Discriminator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "homodynediscriminator",
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

        let mut group = c.benchmark_group("homodynediscriminator_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust HoMoDyne Discriminator", |b| {
            b.iter(|| {
                let result = HomodyneDiscriminator::indicator(&inputs, &[], None)
                    .expect("Homodyne Discriminator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Homodyne Discriminator using `batch_indicator` for streaming updates.
///
/// Seeds state with `min_data` bars then processes the remainder in `CHUNK_SIZE`-bar
/// chunks. Also measures the single-bar update cost.
fn bench_homodynediscriminator_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("homodynediscriminator");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            // --- chunked from-state ---
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let seed = HomodyneDiscriminator::min_data(&[]).max(CHUNK_SIZE);
                    let (_, mut state) =
                        HomodyneDiscriminator::indicator(&[&close[..seed]], &[], None)
                            .expect("Homodyne Discriminator seed failed");

                    let mut chunks = close[seed..].chunks_exact(CHUNK_SIZE);
                    for chunk in chunks.by_ref() {
                        black_box(
                            state
                                .batch_indicator(&[chunk], None)
                                .expect("batch_indicator failed"),
                        );
                    }
                    let rem = chunks.remainder();
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
                "homodynediscriminator",
                "Rust_FromState",
                &[],
                n,
                &timing,
                Some(stock_symbol),
            );

            // --- single-bar update ---
            if n > 1 {
                let (_, mut state) =
                    HomodyneDiscriminator::indicator(&[&close[..n - 1]], &[], None)
                        .expect("Homodyne Discriminator seed (1-bar) failed");
                let final_input = [&close[n - 1..]];

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        black_box(
                            state
                                .batch_indicator(&final_input, None)
                                .expect("Homodyne Discriminator 1-bar update failed"),
                        );
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "homodynediscriminator",
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
        let seed = HomodyneDiscriminator::min_data(&[]).max(CHUNK_SIZE);
        let (_, mut state) = HomodyneDiscriminator::indicator(&[&close_vec[..seed]], &[], None)
            .expect("Homodyne Discriminator seed failed");

        let mut group = c.benchmark_group("homodynediscriminator_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust Homodyne Discriminator from state", |b| {
            b.iter(|| {
                let mut chunks = close_vec[seed..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    black_box(
                        state
                            .batch_indicator(&[chunk], None)
                            .expect("batch_indicator failed"),
                    );
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    black_box(
                        state
                            .batch_indicator(&[rem], None)
                            .expect("batch_indicator failed"),
                    );
                }
            });
        });
        group.finish();

        // Single-bar update bench
        if close_vec.len() > 1 {
            let (_, mut state) =
                HomodyneDiscriminator::indicator(&[&close_vec[..close_vec.len() - 1]], &[], None)
                    .expect("Homodyne Discriminator seed (1-bar) failed");
            let final_input = [&close_vec[close_vec.len() - 1..]];

            let mut group = c.benchmark_group("homodynediscriminator_rust_from_state_1_bar");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("Rust Homodyne Discriminator from state 1 bar", |b| {
                b.iter(|| {
                    black_box(
                        state
                            .batch_indicator(&final_input, None)
                            .expect("Homodyne Discriminator 1-bar update failed"),
                    );
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Homodyne Discriminator for 4 assets simultaneously using SIMD by_assets.
///
/// Processes N=4 assets in a single SIMD pass. The amortised per-asset cost should
/// be lower than four serial scalar runs due to SIMD parallelism across the full
/// four-stage HT cascade and the expensive `atan` call.
fn bench_homodynediscriminator_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("homodynediscriminator");

        let data = get_all_stock_data().unwrap();

        // Take the first 4 stocks for a 4-lane SIMD run.
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, stock)| (symbol.clone(), get_close_array(stock)))
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
                let result = HomodyneDiscriminator::indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("SIMD by_assets Homodyne Discriminator failed");
                black_box(&result);
            },
            SAMPLE_SIZE,
        );
        log_timing_result(
            "homodynediscriminator",
            "Rust_SIMD_by_assets",
            &[],
            stock_data[0].1.len(),
            &timing,
            Some("4_Assets"),
        );
    } else {
        let close_vec = expand_inputs();

        // 4 identical datasets — isolates SIMD overhead from data-layout effects.
        let inputs: [&[&[f64]; 1]; 4] =
            [&[&close_vec], &[&close_vec], &[&close_vec], &[&close_vec]];

        let mut group = c.benchmark_group("homodynediscriminator_rust_simd_by_assets");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by_assets Homodyne Discriminator (N=4)", |b| {
            b.iter(|| {
                let result = HomodyneDiscriminator::indicator_by_assets::<4>(&inputs, &[], None)
                    .expect("SIMD by_assets Homodyne Discriminator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// TA-Lib HT_DCPERIOD — Dominant Cycle Period (no parameters).
///
/// TA-Lib's HT_DCPERIOD is the direct equivalent of this crate's
/// Homodyne Discriminator output.  Lookback is 32 bars.
#[cfg(feature = "talib")]
fn bench_talib_ht_dcperiod(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("homodynediscriminator");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];
            let lookback = ta_ht_dcperiod_start();
            assert!(lookback >= 0);
            let out_len = n - lookback as usize;
            let mut out_dc = vec![0.0_f64; out_len];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut outputs: Vec<*mut f64> = vec![out_dc.as_mut_ptr()];
                    let ret = ta_ht_dcperiod(
                        n as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_ht_dcperiod returned error {ret}");
                    black_box(&out_dc);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "homodynediscriminator",
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
        let lookback = ta_ht_dcperiod_start();
        assert!(lookback >= 0);
        let out_len = n - lookback as usize;
        let mut group = c.benchmark_group("homodynediscriminator_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib HT_DCPERIOD", |b| {
            b.iter(|| {
                let mut out_dc = vec![0.0_f64; out_len];
                let mut outputs: Vec<*mut f64> = vec![out_dc.as_mut_ptr()];
                let ret = ta_ht_dcperiod(
                    n as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_ht_dcperiod returned error {ret}");
                black_box(&out_dc);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_homodynediscriminator_simd_by_assets,
    bench_talib_ht_dcperiod,
    bench_homodynediscriminator,
    bench_homodynediscriminator_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_homodynediscriminator_simd_by_assets,
    bench_homodynediscriminator,
    bench_homodynediscriminator_from_state,
);
criterion_main!(benches);
