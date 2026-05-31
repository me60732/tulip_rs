use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::fisher::{
    indicator, indicator_by_assets, indicator_by_options, min_data, TIndicatorState,
};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_fisher, ti_fisher_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
//const SAMPLE_SIZE: usize = 30000;
// Test data from fisher_test.rs
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 87.01,
    87.87, 87.60,
];

const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 86.54,
    87.66, 87.00,
];

// Options from fisher_test.rs
//const OPTIONS_LIST: [[f64; 1]; 6] = [[10.0], [14.0], [25.0], [35.0], [50.0], [100.0]];
const OPTIONS_LIST: [[f64; 1]; 4] = [[25.0], [35.0], [50.0], [100.0]];
//const OPTIONS_LIST: [[f64; 1]; 2] = [[50.0], [100.0]];
// Chunk size for batched processing
const CHUNK_SIZE: usize = 100;

/// Expand the sample input data by repeating it for synthetic benchmarking
fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..499 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

/// Extract high and low price arrays from stock data
fn get_high_low_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
    let high = stock_data.iter().map(|d| d.high).collect();
    let low = stock_data.iter().map(|d| d.low).collect();
    (high, low)
}

fn bench_c_fisher(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("fisher");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);
            let n = high.len();
            let inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = unsafe { ti_fisher_start(options.as_ptr()) };
                        let output_len = high.len() - (start_index as usize);
                        let mut fisher_output_vec = vec![0.0_f64; output_len];
                        let mut signal_output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![
                            fisher_output_vec.as_mut_ptr(),
                            signal_output_vec.as_mut_ptr(),
                        ];
                        let ret = unsafe {
                            ti_fisher(
                                high.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_fisher returned error code {}", ret);
                        black_box(&fisher_output_vec);
                        black_box(&signal_output_vec);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "fisher",
                    "C_tulip",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group(format!("C Fisher {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let inputs: Vec<*const f64> =
                        vec![black_box(&high).as_ptr(), black_box(&low).as_ptr()];
                    let start_index = unsafe { ti_fisher_start(options.as_ptr()) };
                    let output_len = high.len() - (start_index as usize);
                    let mut fisher_output_vec = vec![0.0_f64; output_len];
                    let mut signal_output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![
                        fisher_output_vec.as_mut_ptr(),
                        signal_output_vec.as_mut_ptr(),
                    ];

                    let ret = unsafe {
                        ti_fisher(
                            high.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_fisher returned error code {}", ret);
                    black_box(&fisher_output_vec);
                    black_box(&signal_output_vec);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_fisher(c: &mut Criterion) {
    if should_log_to_db() {
        // Database logging mode - benchmark real market data
        init_database_data();
        init_logging("fisher");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&[high.as_slice(), low.as_slice()], &options, None);
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result("fisher", "Rust", &options, n, &timing, Some(stock_symbol));
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        println!("Running Rust Fisher criterion mode");
        let (high, low) = expand_inputs();
        println!(
            "Expanded inputs: high len={}, low len={}",
            high.len(),
            low.len()
        );

        for options in OPTIONS_LIST {
            println!("Setting up benchmark group for options: {:?}", options);
            let mut group = c.benchmark_group(format!("Rust Fisher {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);

            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let result = indicator(&[high.as_slice(), low.as_slice()], &options, None);
                    black_box(&result);
                });
            });

            group.finish();
        }
    }
}

fn bench_rust_fisher_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("fisher");

        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                let min_data = min_data(&options);
                timing.measure(
                    || {
                        // First chunk
                        let chunk_inputs = [&high[..min_data], &low[..min_data]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("Fisher indicator failed");

                        // Chunks
                        let mut high_chunks = high[min_data..].chunks_exact(CHUNK_SIZE);
                        let mut low_chunks = low[min_data..].chunks_exact(CHUNK_SIZE);

                        for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref())
                        {
                            let result = state.batch_indicator(&[high_chunk, low_chunk], None);
                            black_box(&result);
                        }

                        // Remainder
                        let high_rem = high_chunks.remainder();
                        let low_rem = low_chunks.remainder();

                        if !high_rem.is_empty() && !low_rem.is_empty() {
                            let result = state.batch_indicator(&[high_rem, low_rem], None);
                            black_box(&result);
                        }
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "fisher",
                    "Rust_FromState",
                    &options,
                    n,
                    &timing,
                    Some(stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if high.len() > 1 {
                    let new_inputs = [&high[..high.len() - 1], &low[..low.len() - 1]];
                    let (_, mut state) = indicator(&new_inputs, &options, None)
                        .expect("Rust Fisher indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let result = state
                                .batch_indicator(
                                    &[&high[high.len() - 1..], &low[low.len() - 1..]],
                                    None,
                                )
                                .expect("Rust Fisher from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "fisher",
                        "Rust_FromState_1_Bar",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );

                    // --- Rust_FromState_1_Bar_json benchmark ---
                    /*let (_, state) = indicator(&new_inputs, &options, None)
                        .expect("Rust Fisher indicator failed");
                    let json = serde_json::to_string(&state).expect("json failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            let result = state
                                .batch_indicator(
                                    &[&high[high.len() - 1..], &low[low.len() - 1..]],
                                    None,
                                )
                                .expect("Rust Fisher from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "fisher",
                        "Rust_FromState_1_Bar_json",
                        &options,
                        n,
                        &timing,
                        Some(stock_symbol),
                    );*/
                }
            }
        }
    } else {
        // Criterion profiling mode - benchmark synthetic data
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            let min_data = min_data(&options);
            // First chunk
            let (_, mut state) = indicator(&[&high[..min_data], &low[..min_data]], &options, None)
                .expect("Fisher indicator failed");

            let mut group =
                c.benchmark_group(format!("Rust Fisher from state {{ {:.1} }}", options[0]));
            group.sample_size(SAMPLE_SIZE);
            group.bench_function("benchmark", |b| {
                b.iter(|| {
                    let mut high_chunks = high[min_data..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let result = state.batch_indicator(&[high_chunk, low_chunk], None);
                        black_box(&result);
                    }

                    // Remainder
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();

                    if !high_rem.is_empty() && !low_rem.is_empty() {
                        let result = state.batch_indicator(&[high_rem, low_rem], None);
                        black_box(&result);
                    }
                });
            });
            group.finish();

            // Benchmark with 1 bar from state
            if high.len() > 1 {
                let (_, mut state) = indicator(
                    &[&high[..high.len() - 1], &low[..low.len() - 1]],
                    &options,
                    None,
                )
                .expect("Rust Fisher indicator failed");

                let mut group = c.benchmark_group(format!(
                    "Rust Fisher from state 1 bar {{ {:.1} }}",
                    options[0]
                ));
                group.sample_size(SAMPLE_SIZE);
                group.bench_function("benchmark", |b| {
                    b.iter(|| {
                        let result = state
                            .batch_indicator(
                                &[&high[high.len() - 1..], &low[low.len() - 1..]],
                                None,
                            )
                            .expect("Rust Fisher from state indicator failed");
                        black_box(&result);
                    });
                });
                group.finish();
            }
        }
    }
}

fn bench_rust_fisher_simd_by_options(c: &mut Criterion) {
    let options_4 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    //let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
    if should_log_to_db() {
        init_database_data();
        init_logging("fisher");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let inputs = [high_vec.as_slice(), low_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    // Process first 4 options with 4-wide SIMD
                    let result = indicator_by_options::<4>(&inputs, &options_4, None)
                        .expect("Rust SIMD Fisher indicator failed");
                    black_box(&result);

                    // Process remaining 2 options with 2-wide SIMD

                    /*let result =
                        tulip_rs::indicators::nightly::fisher_simd::indicator_by_options::<2>(
                            &inputs, &options_2, None,
                        )
                        .expect("Rust SIMD Fisher indicator failed");
                    black_box(&result);*/
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "fisher",
                "Rust_SIMD",
                &[0.0],
                high_vec.len(),
                &timing,
                Some(stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];

        let mut group = c.benchmark_group("fisher_rust_simd_by_options");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD by options Fisher (4 lanes)", |b| {
            b.iter(|| {
                // Process first 4 options with 4-wide SIMD
                let result = indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("Rust SIMD Fisher indicator failed");
                black_box(&result);
            });
        });

        /*group.bench_function("Rust SIMD by options Fisher (2 lanes)", |b| {
            b.iter(|| {
                // Process remaining 2 options with 2-wide SIMD
                let result = tulip_rs::indicators::nightly::fisher_simd::indicator_by_options::<2>(
                    &inputs, &options_2, None,
                )
                .expect("Rust SIMD Fisher indicator failed");
                black_box(&result);
            });
        });*/
        group.finish();
    }
}

fn bench_rust_fisher_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("fisher");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' high/low data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low) = get_high_low_arrays(data);
                (symbol.clone(), high, low)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2],
            &[&stock_data[1].1, &stock_data[1].2],
            &[&stock_data[2].1, &stock_data[2].2],
            &[&stock_data[3].1, &stock_data[3].2],
        ];
        /*let inputs: [&[&[f64]; 2]; 8] = [
            &[&stock_data[0].1, &stock_data[0].2],
            &[&stock_data[1].1, &stock_data[1].2],
            &[&stock_data[2].1, &stock_data[2].2],
            &[&stock_data[3].1, &stock_data[3].2],
            &[&stock_data[4].1, &stock_data[4].2],
            &[&stock_data[5].1, &stock_data[5].2],
            &[&stock_data[6].1, &stock_data[6].2],
            &[&stock_data[7].1, &stock_data[7].2],
        ];*/

        for options in OPTIONS_LIST {
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator_by_assets::<4>(&inputs, &options, None);
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "fisher",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("All"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 2]; 4] = [
            &[high_vec.as_slice(), low_vec.as_slice()],
            &[high_vec.as_slice(), low_vec.as_slice()],
            &[high_vec.as_slice(), low_vec.as_slice()],
            &[high_vec.as_slice(), low_vec.as_slice()],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("fisher_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                format!("Rust SIMD by assets Fisher {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets Fisher indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

criterion_group!(
    fisher_benchmarks,
    bench_rust_fisher_simd_by_options,
    bench_rust_fisher_simd_by_assets,
    bench_rust_fisher,
    bench_c_fisher,
    bench_rust_fisher_from_state,
);
criterion_main!(fisher_benchmarks);
