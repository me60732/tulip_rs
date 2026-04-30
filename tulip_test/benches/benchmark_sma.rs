use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tulip_rs::indicators::sma::{
    indicator, indicator_by_assets, indicator_by_options, min_data, IndicatorState, TIndicatorState,
};

use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
const SAMPLE_SIZE: usize = 1000000;
use tulip_test::c_bindings::{ti_sma, ti_sma_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
use tulip_test::talib_bindings::{ta_sma, ta_sma_start};

// Sample input data from sma_test.rs
const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];

// Options for SMA (period)
const OPTIONS_LIST: [[f64; 1]; 8] = [[10.0], [14.0], [20.0], [30.0], [50.0], [100.0], [200.0], [300.0]];

// Chunk size for from_state benchmarks
const CHUNK_SIZE: usize = 100;

fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..500 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

/// Benchmark the C implementation of SMA.
fn bench_c_sma(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let start_index = unsafe { ti_sma_start(options.as_ptr()) };
                        assert!(start_index >= 0, "ti_sma_start returned a negative index");
                        let output_len = close_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                        let ret = unsafe {
                            ti_sma(
                                close_vec.len() as i32,
                                inputs.as_ptr(),
                                options.as_ptr(),
                                outputs.as_mut_ptr(),
                            )
                        };
                        assert_eq!(ret, 0, "ti_sma returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "sma",
                    "C_tulip",
                    &options,
                    close_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = unsafe { ti_sma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_sma_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("sma_c");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("C SMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_sma(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_sma returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust implementation of SMA.
fn bench_rust_sma(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [close_vec.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        indicator(&inputs, &options, None).expect("Rust SMA indicator failed");
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "sma",
                    "Rust",
                    &options,
                    close_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("sma_rust");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("Rust SMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    indicator(&inputs, &options, None).expect("Rust SMA indicator failed");
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust from_state implementation of SMA.
fn bench_rust_sma_from_state(_c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();

            let new_inputs = [&close_vec[..close_vec.len() - 1]];
            let final_inputs = [&close_vec[close_vec.len() - 1..]];
            let inputs = [close_vec.as_slice()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let min_data_val = min_data(&options).max(CHUNK_SIZE);
                        // First chunk
                        let chunk_inputs = [&close_vec[..min_data_val]];

                        let (_, mut state) = indicator(&chunk_inputs, &options, None)
                            .expect("Rust SMA indicator failed");

                        // Chunks
                        let mut close_chunks = close_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                        for close_chunk in close_chunks.by_ref() {
                            let chunk_inputs = [close_chunk];
                            let result = state.batch_indicator(&chunk_inputs, None);
                            black_box(&result);
                        }

                        // Remainder
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
                    "sma",
                    "Rust_FromState",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_symbol),
                );

                // --- Rust_FromState_1_Bar benchmark ---
                if inputs[0].len() > 1 {
                    let (_, state) =
                        indicator(&new_inputs, &options, None).expect("Rust SMA indicator failed");
                    //let bin = bincode::serde::encode_to_vec(&state, bincode::config::standard()).expect("bincode encode failed");
                    let json = serde_json::to_string(&state).expect("json failed");
                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            let mut state: IndicatorState =
                                serde_json::from_str(&json).expect("JSON failed");
                            //let (mut state, _): (IndicatorState, _) = bincode::serde::decode_from_slice(&bin, bincode::config::standard()).expect("bincode decode failed");
                            let result = state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust SMA from state indicator failed");
                            black_box(&result);
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "sma",
                        "Rust_FromState_1_Bar_json",
                        &options,
                        inputs[0].len(),
                        &timing,
                        Some(&stock_symbol),
                    );

                    let (_, mut state) =
                        indicator(&new_inputs, &options, None).expect("Rust SMA indicator failed");

                    let mut timing = TimingMeasurements::new();
                    timing.measure(
                        || {
                            state
                                .batch_indicator(&final_inputs, None)
                                .expect("Rust SMA from state indicator failed");
                        },
                        SAMPLE_SIZE,
                    );

                    log_timing_result(
                        "sma",
                        "Rust_FromState_1_Bar",
                        &options,
                        inputs[0].len(),
                        &timing,
                        Some(&stock_symbol),
                    );
                }
            }
        }
    }
}

/// Benchmark the TA-Lib implementation of SMA.
fn bench_talib_sma(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

            for options in OPTIONS_LIST {
                let mut timing = TimingMeasurements::new();

                timing.measure(
                    || {
                        let start_index = ta_sma_start(options[0]);
                        assert!(start_index >= 0, "ta_sma_start returned a negative index");
                        let output_len = close_vec.len() - (start_index as usize);
                        let mut output_vec = vec![0.0_f64; output_len];
                        let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];
                        let ret = ta_sma(
                            close_vec.len() as i32,
                            inputs.as_ptr(),
                            options.as_ptr(),
                            outputs.as_mut_ptr(),
                        );
                        assert_eq!(ret, 0, "ta_sma returned error code {}", ret);
                        black_box(&output_vec);
                    },
                    SAMPLE_SIZE,
                );

                log_timing_result(
                    "sma",
                    "talib",
                    &options,
                    close_vec.len(),
                    &timing,
                    Some(&stock_symbol),
                );
            }
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs: Vec<*const f64> = vec![close_vec.as_ptr()];

        for options in OPTIONS_LIST {
            let start_index = ta_sma_start(options[0]);
            assert!(start_index >= 0, "ta_sma_start returned a negative index");
            let output_len = close_vec.len() - (start_index as usize);

            let mut group = c.benchmark_group("sma_talib");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(&format!("TA-Lib SMA {{ {} }}", options[0]), |b| {
                b.iter(|| {
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = ta_sma(
                        close_vec.len() as i32,
                        inputs.as_ptr(),
                        options.as_ptr(),
                        outputs.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ta_sma returned error code {}", ret);
                    black_box(&output_vec);
                });
            });
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD by assets implementation of SMA.
fn bench_rust_sma_simd_by_assets(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
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
                        .expect("Rust SIMD by assets SMA indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "sma",
                "Rust_SIMD_by_assets",
                &options,
                stock_data[0].1.len(),
                &timing,
                Some("All"),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();

        // Create 4 identical datasets for SIMD processing
        let inputs: [&[&[f64]; 1]; 4] = [
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
            &[close_vec.as_slice()],
        ];

        for options in OPTIONS_LIST {
            let mut group = c.benchmark_group("sma_rust_simd_by_assets");
            group.sample_size(SAMPLE_SIZE);
            group.bench_function(
                &format!("Rust SIMD by assets SMA {{ {} }}", options[0]),
                |b| {
                    b.iter(|| {
                        let result = indicator_by_assets::<4>(&inputs, &options, None)
                            .expect("Rust SIMD by assets SMA indicator failed");
                        black_box(&result);
                    });
                },
            );
            group.finish();
        }
    }
}

/// Benchmark the Rust SIMD implementation of SMA.
//#[cfg(feature = "portable_simd")]
fn bench_rust_sma_simd(c: &mut Criterion) {
    let options_4_1 = [
        &OPTIONS_LIST[0],
        &OPTIONS_LIST[1],
        &OPTIONS_LIST[2],
        &OPTIONS_LIST[3],
    ];
    let options_4_2 = [
        &OPTIONS_LIST[4],
        &OPTIONS_LIST[5],
        &OPTIONS_LIST[6],
        &OPTIONS_LIST[7],
    ];
    if should_log_to_db() {
        init_database_data();
        init_logging("sma");

        let data = get_all_stock_data().unwrap();
        let options_8 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
            &OPTIONS_LIST[4],
            &OPTIONS_LIST[5],
            &OPTIONS_LIST[6],
            &OPTIONS_LIST[7],
        ];
        for (stock_symbol, stock_data) in data {
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
            let inputs = [close_vec.as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result_8 = indicator_by_options::<8>(&inputs, &options_8, None)
                        .expect("Rust SIMD SMA indicator failed");
                    black_box(&result_8);
                    
                    /*let result_4 = indicator_by_options::<4>(&inputs, &options_4_1, None)
                        .expect("Rust SIMD SMA indicator failed");
                    black_box(&result_4);

                    let result_2 = indicator_by_options::<4>(&inputs, &options_4_2, None)
                        .expect("Rust SIMD SMA indicator failed");
                    black_box(&result_2);*/
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "sma",
                "Rust_SIMD",
                &[0.0],
                close_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let close_vec = expand_inputs();
        let inputs = [close_vec.as_slice()];

        let mut group = c.benchmark_group("sma_rust_simd");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust SIMD SMA (4+2 lanes)", |b| {
            b.iter(|| {

                let result_4 = indicator_by_options::<4>(&inputs, &options_4_1, None)
                    .expect("Rust SIMD SMA indicator failed");
                black_box(&result_4);

                let result_2 = indicator_by_options::<4>(&inputs, &options_4_2, None)
                    .expect("Rust SIMD SMA indicator failed");
                black_box(&result_2);
            });
        });
        group.finish();
    }
}

//REPLACE WITH TEST FUNCTIONS

criterion_group!(
    benches,
    bench_rust_sma_simd,
    bench_rust_sma_simd_by_assets,
    bench_rust_sma,
    bench_c_sma,
    bench_rust_sma_from_state,
    bench_talib_sma,
);
criterion_main!(benches);
