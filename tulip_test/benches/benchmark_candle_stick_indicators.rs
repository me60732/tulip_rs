use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::indicators::candlestick::{indicator, min_data, ForcastType, OPTIONS_WIDTH};
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
//use tulip_test::benchmark_utils::SAMPLE_SIZE;
const SAMPLE_SIZE: usize = 10000;
#[cfg(feature = "perf-stats")]
use tulip_rs::candle_indicators::perf_stats::PERF_COUNTERS;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};
// Sample input data
const OPEN: [f64; 15] = [
    81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
    88.00, 87.60,
];
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

// Default options: [candle_period, trend_period, trend_signal_period]
const OPTIONS: [f64; OPTIONS_WIDTH] = [5.0, 10.0, 3.0];

// Utility function to expand data
fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open_vec = OPEN.to_vec();
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..200 {
        open_vec.extend_from_slice(&OPEN);
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        close_vec.extend_from_slice(&CLOSE);
    }
    (open_vec, high_vec, low_vec, close_vec)
}

/// Get all forecast types including None
fn get_all_forecast_variants() -> Vec<(String, Option<ForcastType>)> {
    vec![
        (
            "BearishReversal".to_string(),
            Some(ForcastType::BearishReversal),
        ),
        (
            "BullishReversal".to_string(),
            Some(ForcastType::BullishReversal),
        ),
        (
            "BearishContinuation".to_string(),
            Some(ForcastType::BearishContinuation),
        ),
        (
            "BullishContinuation".to_string(),
            Some(ForcastType::BullishContinuation),
        ),
        (
            "BearishReversalOrContinuation".to_string(),
            Some(ForcastType::BearishReversalOrContinuation),
        ),
        (
            "BullishReversalOrContinuation".to_string(),
            Some(ForcastType::BullishReversalOrContinuation),
        ),
        ("None".to_string(), None),
    ]
}

/// Benchmark the Rust implementation of candlestick indicator for all forecast types.
fn bench_rust_candlestick(c: &mut Criterion) {
    if should_log_to_db() {
        init_database_data();
        init_logging("candlestick");

        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let open_vec: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
            let high_vec: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
            let low_vec: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
            let close_vec: Vec<f64> = stock_data.iter().map(|d| d.close).collect();

            // Skip if not enough data
            let min_required = min_data(&OPTIONS);
            if open_vec.len() < min_required {
                continue;
            }

            let inputs = [
                open_vec.as_slice(),
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            // Benchmark each forecast type
            for (forecast_name, forecast_type) in get_all_forecast_variants() {
                /*#[cfg(feature = "perf-stats")]
                PERF_COUNTERS.reset();

                // Run just ONE iteration to get single-run stats
                #[cfg(feature = "perf-stats")]
                {
                    let result = indicator(&inputs, &OPTIONS, forecast_type)
                        .expect("Rust candlestick indicator failed");
                    black_box(&result);

                    println!("Stock Symbol: {:?}", stock_symbol);
                    let stats = PERF_COUNTERS.snapshot();
                    eprintln!("\nStats for {} (single iteration):", forecast_name);
                    stats.print_summary();
                    PERF_COUNTERS.reset(); // Clear the single-run stats
                                           //break;
                }*/

                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = indicator(&inputs, &OPTIONS, forecast_type)
                            .expect("Rust candlestick indicator failed");
                        black_box(&result);
                    },
                    SAMPLE_SIZE,
                );
                log_timing_result(
                    "Rust_Candlestick",
                    &forecast_name,
                    &OPTIONS,
                    inputs[0].len(),
                    &timing,
                    Some(stock_symbol),
                );
            }
            break;
        }

        // Print performance statistics if enabled
        #[cfg(feature = "perf-stats")]
        {
            use tulip_rs::candle_indicators::perf_stats::PERF_COUNTERS;
            let stats = PERF_COUNTERS.snapshot();
            eprintln!("\n{}", "=".repeat(70));
            eprintln!("PERFORMANCE STATISTICS (DB logging run)");
            stats.print_summary();
        }
    } else {
        // Run Criterion benchmark with synthetic data
        let (open_vec, high_vec, low_vec, close_vec) = expand_inputs();
        let inputs = [
            open_vec.as_slice(),
            high_vec.as_slice(),
            low_vec.as_slice(),
            close_vec.as_slice(),
        ];

        let mut group = c.benchmark_group("candlestick_rust");
        group.sample_size(SAMPLE_SIZE);

        for (forecast_name, forecast_type) in get_all_forecast_variants() {
            group.bench_function(&format!("Rust Candlestick - {}", forecast_name), |b| {
                b.iter(|| {
                    let result = indicator(&inputs, &OPTIONS, forecast_type)
                        .expect("Rust candlestick indicator failed");
                    black_box(&result);
                });
            });
        }

        group.finish();

        // Print performance statistics if enabled
        #[cfg(feature = "perf-stats")]
        {
            use tulip_rs::candle_indicators::perf_stats::PERF_COUNTERS;
            let stats = PERF_COUNTERS.snapshot();
            eprintln!("\n{}", "=".repeat(70));
            eprintln!("PERFORMANCE STATISTICS (Criterion run)");
            stats.print_summary();
        }
    }
}

criterion_group!(benches, bench_rust_candlestick);
criterion_main!(benches);

// Print performance stats after all benchmarks complete
#[cfg(feature = "perf-stats")]
#[cfg(test)]
mod perf_stats_printer {
    use tulip_rs::candle_indicators::perf_stats::PERF_COUNTERS;

    #[test]
    fn print_stats() {
        // This won't actually run in criterion, but we can add a Drop impl
    }
}

// Alternative: Add stats printing in the benchmark itself
// This will print after criterion finishes
