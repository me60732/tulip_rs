use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::ao_medprice::{indicator, min_data, IndicatorState, TIndicatorState};
use tulip_rs::indicators::medprice::indicator as medprice_indicator;
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::c_bindings::{ti_ao, ti_ao_start};
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

// Sample input data from ao_test.rs
const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

// Options are empty for AO
const OPTIONS: [f64; 0] = [];

fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..500 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

/// Benchmark the C implementation of AO.
fn bench_c_ao(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ao_medprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let start_index = unsafe { ti_ao_start(OPTIONS.as_ptr()) };
                    assert!(start_index >= 0, "ti_ao_start returned a negative index");
                    let output_len = high_vec.len() - (start_index as usize);
                    let mut output_vec = vec![0.0_f64; output_len];
                    let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                    let ret = unsafe {
                        ti_ao(
                            high_vec.len() as i32,
                            inputs.as_ptr(),
                            OPTIONS.as_ptr(),
                            outputs.as_mut_ptr(),
                        )
                    };
                    assert_eq!(ret, 0, "ti_ao returned error code {}", ret);
                    black_box(&output_vec);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "C_tulip",
                &OPTIONS,
                high_vec.len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs: Vec<*const f64> = vec![high_vec.as_ptr(), low_vec.as_ptr()];

        let start_index = unsafe { ti_ao_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_ao_start returned a negative index");
        let output_len = high_vec.len() - (start_index as usize);

        let mut group = c.benchmark_group("ao_c");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("C AO", |b| {
            b.iter(|| {
                let mut output_vec = vec![0.0_f64; output_len];
                let mut outputs: Vec<*mut f64> = vec![output_vec.as_mut_ptr()];

                let ret = unsafe {
                    ti_ao(
                        high_vec.len() as i32,
                        inputs.as_ptr(),
                        OPTIONS.as_ptr(),
                        outputs.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_ao returned error code {}", ret);
                black_box(&output_vec);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust implementation of AO.
fn bench_rust_ao(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ao_medprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let inputs = [high_vec.as_slice(), low_vec.as_slice()];

            let medprice_result =
                medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
            let medprice = [medprice_result.0[0].as_slice()];

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result =
                        indicator(&medprice, &OPTIONS, None).expect("Rust AO indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "Rust",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];
        let medprice_result =
            medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
        let medprice = [medprice_result.0[0].as_slice()];
        let mut group = c.benchmark_group("ao_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust AO", |b| {
            b.iter(|| {
                let result =
                    indicator(&medprice, &OPTIONS, None).expect("Rust AO indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust implementation of AO with optional outputs.
fn bench_rust_ao_optional(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ao_medprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let inputs = [high_vec.as_slice(), low_vec.as_slice()];
            let medprice_result =
                medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
            let medprice = [medprice_result.0[0].as_slice()];
            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = indicator(&medprice, &OPTIONS, Some(&[true, true, true]))
                        .expect("Rust AO indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "Rust_optional",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];
        let medprice_result =
            medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
        let medprice = [medprice_result.0[0].as_slice()];
        let mut group = c.benchmark_group("ao_rust");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust AO", |b| {
            b.iter(|| {
                let result = indicator(&medprice, &OPTIONS, Some(&[true, true]))
                    .expect("Rust AO indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

/// Benchmark the Rust from_state implementation of AO.
const CHUNK_SIZE: usize = 100;
fn bench_rust_ao_from_state(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("ao_medprice");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let inputs = [high.as_slice(), low.as_slice()];
            let medprice_result =
                medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
            let medprice = medprice_result.0[0].as_slice();
            let mut timing = TimingMeasurements::new();

            timing.measure(
                || {
                    let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
                    // First chunk
                    let chunk_inputs = [&medprice[..min_data_val]];

                    let (_, mut state) =
                        indicator(&chunk_inputs, &OPTIONS, None).expect("DX indicator failed");

                    // Chunks
                    let mut med_chunks = medprice[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for med_chunk in med_chunks.by_ref() {
                        let chunk_inputs = [med_chunk];
                        let result = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("AO INDICATOR failed");
                        black_box(&result);
                    }

                    // Remainder
                    let med_rem = med_chunks.remainder();

                    if !med_rem.is_empty() {
                        let chunk_inputs = [med_rem];
                        let result = state.batch_indicator(&chunk_inputs, None);
                        black_box(&result);
                    }
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "Rust_FromState",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );

            let new_inputs = [&medprice[..medprice.len() - 1]];
            let final_inputs = [&medprice[medprice.len() - 1..]];

            let (_, mut state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust AO indicator failed");

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust AO from state indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "Rust_FromState_1_Bar",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );

            let (_, state) =
                indicator(&new_inputs, &OPTIONS, None).expect("Rust AO indicator failed");
            let json = serde_json::to_string(&state).expect("json failed");

            let mut timing = TimingMeasurements::new();
            timing.measure(
                || {
                    let mut state: IndicatorState =
                        serde_json::from_str(&json).expect("JSON failed");
                    let result = state
                        .batch_indicator(&final_inputs, None)
                        .expect("Rust AO from state indicator failed");
                    black_box(&result);
                },
                SAMPLE_SIZE,
            );

            log_timing_result(
                "ao_medprice",
                "Rust_FromState_1_Bar_json",
                &OPTIONS,
                inputs[0].len(),
                &timing,
                Some(&stock_symbol),
            );
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (high_vec, low_vec) = expand_inputs();
        let inputs = [high_vec.as_slice(), low_vec.as_slice()];
        let medprice_result =
            medprice_indicator(&inputs, &[], None).expect("Medprice Indicator Failed");
        let medprice = [medprice_result.0[0].as_slice()];
        let (_, mut state) =
            indicator(&medprice, &OPTIONS, None).expect("Rust AO indicator failed");

        let mut group = c.benchmark_group("ao_rust_from_state");
        group.sample_size(SAMPLE_SIZE);
        group.bench_function("Rust AO from state", |b| {
            b.iter(|| {
                let result = state
                    .batch_indicator(&medprice, None)
                    .expect("Rust AO from state indicator failed");
                black_box(&result);
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_c_ao,
    bench_rust_ao,
    bench_rust_ao_from_state,
    bench_rust_ao_optional
);
criterion_main!(benches);
