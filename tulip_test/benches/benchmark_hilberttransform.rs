use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::hilberttransform::{
    indicator_by_assets, indicator_by_options, HilbertTransform, Indicator, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
#[cfg(feature = "talib")]
use tulip_test::talib_bindings::{ta_ht_phasor, ta_ht_phasor_start};

const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

/// [ss_period, hp_period]
const OPTIONS_LIST: [[f64; 2]; 4] = [[10.0, 48.0], [14.0, 40.0], [20.0, 5.0], [20.0, 60.0]];

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

/// Benchmark the full HilbertTransform indicator (in_phase + quadrature only).
fn bench_rust_hilberttransform(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = HilbertTransform::indicator(&inputs, &options, None)
                            .expect("HilbertTransform failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "hilberttransform",
                    "Rust",
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

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("hilberttransform_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust HilbertTransform {{ ss: {}, hp: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = HilbertTransform::indicator(&inputs, &options, None)
                            .expect("HilbertTransform failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark HilbertTransform with both optional outputs (roofing + highpass) enabled.
fn bench_rust_hilberttransform_with_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");

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
                            HilbertTransform::indicator(&inputs, &options, Some(&[true, true]))
                                .expect("HilbertTransform (with optional) failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "hilberttransform",
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

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("hilberttransform_rust_optional");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust HilbertTransform optional {{ ss: {}, hp: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result =
                            HilbertTransform::indicator(&inputs, &options, Some(&[true, true]))
                                .expect("HilbertTransform (with optional) failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark HilbertTransform using `batch_indicator` for streaming updates.
///
/// Seeds state with `min_data` bars then processes the remainder in
/// `CHUNK_SIZE`-bar chunks. Also measures the single-bar update cost.
fn bench_rust_hilberttransform_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();

            for options in OPTIONS_LIST {
                // --- chunked from-state ---
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let seed = HilbertTransform::min_data(&options).max(CHUNK_SIZE);
                        let (_, mut state) =
                            HilbertTransform::indicator(&[&close[..seed]], &options, None)
                                .expect("HilbertTransform seed failed");

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
                    "hilberttransform",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- single-bar update ---
                if n > 1 {
                    let (_, mut state) =
                        HilbertTransform::indicator(&[&close[..n - 1]], &options, None)
                            .expect("HilbertTransform seed (1-bar) failed");
                    let final_input = [&close[n - 1..]];

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            black_box(
                                state
                                    .batch_indicator(&final_input, None)
                                    .expect("HilbertTransform 1-bar update failed"),
                            );
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "hilberttransform",
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
            let seed = HilbertTransform::min_data(&options).max(CHUNK_SIZE);
            let (_, mut state) = HilbertTransform::indicator(&[&close_vec[..seed]], &options, None)
                .expect("HilbertTransform seed failed");

            let mut group = c.benchmark_group("hilberttransform_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust HilbertTransform from state {{ ss: {}, hp: {} }}",
                    options[0], options[1]
                ),
                |b| {
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
                },
            );
            group.finish();

            // Single-bar update bench
            if close_vec.len() > 1 {
                let (_, mut state) = HilbertTransform::indicator(
                    &[&close_vec[..close_vec.len() - 1]],
                    &options,
                    None,
                )
                .expect("HilbertTransform seed (1-bar) failed");
                let final_input = [&close_vec[close_vec.len() - 1..]];

                let mut group = c.benchmark_group("hilberttransform_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!(
                        "Rust HilbertTransform from state 1 bar {{ ss: {}, hp: {} }}",
                        options[0], options[1]
                    ),
                    |b| {
                        b.iter(|| {
                            black_box(
                                state
                                    .batch_indicator(&final_input, None)
                                    .expect("HilbertTransform 1-bar update failed"),
                            );
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark SIMD by-assets HilbertTransform (4 assets simultaneously).
fn bench_rust_hilberttransform_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");

        let data = get_all_stock_data().unwrap();
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(8)
            .map(|(symbol, eod)| (symbol.clone(), get_close_array(eod)))
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
                        .expect("SIMD by-assets HilbertTransform failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "hilberttransform",
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
            let mut group = c.benchmark_group("hilberttransform_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust HilbertTransform SIMD by assets {{ ss: {}, hp: {} }}",
                    options[0], options[1]
                ),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("SIMD by-assets HilbertTransform failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark SIMD by-options HilbertTransform (4 option sets simultaneously).
fn bench_rust_hilberttransform_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");

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
                        .expect("SIMD by-options HilbertTransform failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "hilberttransform",
                "Rust_SIMD",
                &[0.0, 0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close = expand_inputs();
        let inputs = [close.as_slice()];
        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        let mut group = c.benchmark_group("hilberttransform_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust HilbertTransform SIMD by options (4 lanes)", |b| {
            b.iter(|| {
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by-options HilbertTransform failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// TA-Lib HT_PHASOR — raw Hilbert Transform phasor (no SuperSmoother/HighPass stages).
///
/// TA-Lib outputs In-Phase and Quadrature directly from the Hilbert Transform.
/// Our `hilberttransform` indicator adds a SuperSmoother pre-filter and an
/// optional HighPass stage; this comparison isolates that overhead.
/// Lookback is 32 bars; no parameters.
#[cfg(feature = "talib")]
fn bench_talib_ht_phasor(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("hilberttransform");
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let inputs: Vec<*const f64> = vec![close.as_ptr()];
            let lookback = ta_ht_phasor_start();
            assert!(lookback >= 0);
            let out_len = n - lookback as usize;
            let mut out_ip = vec![0.0_f64; out_len];
            let mut out_q = vec![0.0_f64; out_len];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut outputs: Vec<*mut f64> = vec![out_ip.as_mut_ptr(), out_q.as_mut_ptr()];
                    let ret = ta_ht_phasor(
                        n as i32,
                        inputs.as_ptr(),
                        std::ptr::null(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_ht_phasor returned error {ret}");
                    black_box(&out_ip);
                    black_box(&out_q);
                },
                SAMPLE_SIZE,
            );
            log_timing_result(
                "hilberttransform",
                "talib",
                &[0.0, 0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let close = expand_inputs();
        let n = close.len();
        let inputs: Vec<*const f64> = vec![close.as_ptr()];
        let lookback = ta_ht_phasor_start();
        assert!(lookback >= 0);
        let out_len = n - lookback as usize;
        let mut group = c.benchmark_group("hilberttransform_talib");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("TA-Lib HT_PHASOR (In-Phase + Quadrature)", |b| {
            b.iter(|| {
                let mut out_ip = vec![0.0_f64; out_len];
                let mut out_q = vec![0.0_f64; out_len];
                let mut outputs: Vec<*mut f64> = vec![out_ip.as_mut_ptr(), out_q.as_mut_ptr()];
                let ret = ta_ht_phasor(
                    n as i32,
                    inputs.as_ptr(),
                    std::ptr::null(),
                    outputs.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ta_ht_phasor returned error {ret}");
                black_box(&out_ip);
                black_box(&out_q);
            });
        });
        group.finish();
    }
}

#[cfg(feature = "talib")]
criterion_group!(
    benches,
    bench_rust_hilberttransform_simd_by_assets,
    bench_rust_hilberttransform_simd_by_options,
    bench_rust_hilberttransform,
    bench_talib_ht_phasor,
    bench_rust_hilberttransform_with_optional,
    bench_rust_hilberttransform_from_state,
);

#[cfg(not(feature = "talib"))]
criterion_group!(
    benches,
    bench_rust_hilberttransform_simd_by_assets,
    bench_rust_hilberttransform_simd_by_options,
    bench_rust_hilberttransform,
    bench_rust_hilberttransform_with_optional,
    bench_rust_hilberttransform_from_state,
);
criterion_main!(benches);
