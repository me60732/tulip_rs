use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::donchianchannel::{DonchianChannel, Indicator, TIndicatorState, IndicatorState, indicator_by_assets, indicator_by_options};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_max, ti_max_start, ti_min};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data (high and low prices)
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

// Options for Donchian Channel: [period]
const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];
//const OPTIONS_LIST: [[f64; 1]; 12] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0], [35.0], [50.0], [70.0], [100.0], [150.0], [200.0]];

// Chunk size for batched (from-state) processing
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it to create a realistic dataset.
fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

/// Extract high and low price arrays from stock data rows.
fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    (high, low)
}

/// Benchmark the C Tulip equivalent of Donchian Channel.
///
/// There is no native `ti_donchianchannel`, so this runs `ti_max(high, period)`,
/// `ti_min(low, period)`, then folds a medprice pass `(upper + lower) / 2` over
/// the two output buffers — exactly what the Rust indicator does internally.
fn bench_c_donchianchannel(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("donchianchannel");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start = unsafe { ti_max_start(options.as_ptr()) } as usize;
                        let output_len = n - start;

                        let mut upper = vec![0.0_f64; output_len];
                        let mut lower = vec![0.0_f64; output_len];

                        unsafe {
                            let inputs: Vec<*const f64> = vec![high.as_ptr()];
                            let mut outputs: Vec<*mut f64> = vec![upper.as_mut_ptr()];
                            ti_max(
                                n as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            );

                            let inputs: Vec<*const f64> = vec![low.as_ptr()];
                            let mut outputs: Vec<*mut f64> = vec![lower.as_mut_ptr()];
                            ti_min(
                                n as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            );
                        }

                        // medprice fold: middle = (upper + lower) / 2
                        let middle: Vec<f64> = upper
                            .iter()
                            .zip(lower.iter())
                            .map(|(&u, &l)| (u + l) / 2.0)
                            .collect();

                        black_box(&upper);
                        black_box(&lower);
                        black_box(&middle);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "donchianchannel",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();
        let n = high_vec.len();

        for options in OPTIONS_LIST {
            let start = unsafe { ti_max_start(options.as_ptr()) } as usize;
            let output_len = n - start;

            let mut group = c.benchmark_group("donchianchannel_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("C Donchian Channel {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let mut upper = vec![0.0_f64; output_len];
                        let mut lower = vec![0.0_f64; output_len];

                        unsafe {
                            let inputs: Vec<*const f64> = vec![high_vec.as_ptr()];
                            let mut outputs: Vec<*mut f64> = vec![upper.as_mut_ptr()];
                            ti_max(
                                n as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            );

                            let inputs: Vec<*const f64> = vec![low_vec.as_ptr()];
                            let mut outputs: Vec<*mut f64> = vec![lower.as_mut_ptr()];
                            ti_min(
                                n as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            );
                        }

                        // medprice fold: middle = (upper + lower) / 2
                        let middle: Vec<f64> = upper
                            .iter()
                            .zip(lower.iter())
                            .map(|(&u, &l)| (u + l) / 2.0)
                            .collect();

                        black_box(&upper);
                        black_box(&lower);
                        black_box(&middle);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of Donchian Channel.
fn bench_rust_donchianchannel(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("donchianchannel");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = DonchianChannel::indicator(&inputs, &options, None)
                            .expect("Rust Donchian Channel indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "donchianchannel",
                    "Rust",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("donchianchannel_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust Donchian Channel {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = DonchianChannel::indicator(&inputs, &options, None)
                            .expect("Rust Donchian Channel indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust streaming (from-state) implementation of Donchian Channel.
fn bench_rust_donchianchannel_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("donchianchannel");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                // --- Rust_FromState (chunked) ---
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = DonchianChannel::min_data(&options).max(CHUNK_SIZE);
                        let chunk_inputs = [&high[..min_data_val], &low[..min_data_val]];
                        let (_, mut state) =
                            DonchianChannel::indicator(&chunk_inputs, &options, None)
                                .expect("Donchian Channel indicator failed");

                        let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (hc, lc) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                            let result = state.batch_indicator(&[hc, lc], None);
                            black_box(&result);
                        }

                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        if !high_rem.is_empty() {
                            let result = state.batch_indicator(&[high_rem, low_rem], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "donchianchannel",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                if high.len() > 1 {
                    let new_inputs = [&high[..high.len() - 1], &low[..low.len() - 1]];
                    let final_inputs = [&high[high.len() - 1..], &low[low.len() - 1..]];

                    // --- Rust_FromState_1_Bar ---
                    let (_, mut state) = DonchianChannel::indicator(&new_inputs, &options, None)
                        .expect("Donchian Channel indicator failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Donchian Channel from-state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "donchianchannel",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json ---
                    let (_, state) = DonchianChannel::indicator(&new_inputs, &options, None)
                        .expect("Donchian Channel indicator failed");
                    let json = serde_json::to_string(&state).expect("JSON serialisation failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON deserialisation failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Donchian Channel from-state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );
                    log_timing_result(
                        "donchianchannel",
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
        let (high_vec, low_vec) = expand_inputs();

        for options in OPTIONS_LIST {
            let min_data_val = DonchianChannel::min_data(&options).max(CHUNK_SIZE);
            let chunk_inputs = [&high_vec[..min_data_val], &low_vec[..min_data_val]];
            let (_, mut state) = DonchianChannel::indicator(&chunk_inputs, &options, None)
                .expect("Donchian Channel indicator failed");

            let mut group = c.benchmark_group("donchianchannel_rust_from_state");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!(
                    "Rust Donchian Channel from state {{ period: {} }}",
                    options[0]
                ),
                |b| {
                    b.iter(|| {
                        let mut high_chunks = high_vec[min_data_val..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for (hc, lc) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                            let result = state.batch_indicator(&[hc, lc], None);
                            black_box(&result);
                        }

                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();
                        if !high_rem.is_empty() {
                            let result = state.batch_indicator(&[high_rem, low_rem], None);
                            black_box(&result);
                        }
                    });
                },
            );
            group.finish();

            // Benchmark with 1 bar from state
            if high_vec.len() > 1 {
                let new_inputs = [
                    &high_vec[..high_vec.len() - 1],
                    &low_vec[..low_vec.len() - 1],
                ];
                let final_inputs = [
                    &high_vec[high_vec.len() - 1..],
                    &low_vec[low_vec.len() - 1..],
                ];
                let (_, mut state) = DonchianChannel::indicator(&new_inputs, &options, None)
                    .expect("Donchian Channel indicator failed");

                let mut group = c.benchmark_group("donchianchannel_rust_from_state_1_bar");
                group.sample_size(SAMPLE_SIZE);
                group.bench_function(
                    format!(
                        "Rust Donchian Channel from state 1 bar {{ period: {} }}",
                        options[0]
                    ),
                    |b| {
                        b.iter(|| {
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Donchian Channel from-state indicator failed");
                            black_box(&result);
                        });
                    },
                );
                group.finish();
            }
        }
    }
}

/// Benchmark the Rust SIMD by-assets implementation of Donchian Channel.
fn bench_rust_donchianchannel_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("donchianchannel");

        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low) = get_arrays(eod);
                (symbol.clone(), high, low)
            })
            .collect();

        let asset0: [&[f64]; 2] = [&stock_data[0].1, &stock_data[0].2];
        let asset1: [&[f64]; 2] = [&stock_data[1].1, &stock_data[1].2];
        let asset2: [&[f64]; 2] = [&stock_data[2].1, &stock_data[2].2];
        let asset3: [&[f64]; 2] = [&stock_data[3].1, &stock_data[3].2];
        let inputs: [&[&[f64]; 2]; 4] = [&asset0, &asset1, &asset2, &asset3];

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None)
                        .expect("Rust SIMD by assets DC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "donchianchannel",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("4_Assets"),
            );
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();

        let asset: [&[f64]; 2] = [&high_vec, &low_vec];
        let inputs: [&[&[f64]; 2]; 4] = [&asset, &asset, &asset, &asset];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("donchianchannel_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust SIMD by assets DC {{ period: {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets DC indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by-options implementation of Donchian Channel.
fn bench_rust_donchianchannel_simd_by_options(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("donchianchannel");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];
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
                        .expect("Rust SIMD by options DC indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "donchianchannel",
                "Rust_SIMD",
                &[0.0],
                n,
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        let mut group = c.benchmark_group("donchianchannel_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options DC (4 lanes)", |b| {
            b.iter(|| {
                let options_4 = [
                    &OPTIONS_LIST[0],
                    &OPTIONS_LIST[1],
                    &OPTIONS_LIST[2],
                    &OPTIONS_LIST[3],
                ];
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD by options DC indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_rust_donchianchannel_simd_by_assets,
    bench_rust_donchianchannel_simd_by_options,
    bench_rust_donchianchannel,
    bench_c_donchianchannel,
    bench_rust_donchianchannel_from_state,
);
criterion_main!(benches);
