/*use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::candle_indicators::*;
use tulip_test::benchmark_logger::{init_logging, log_timing_result, should_log_to_db};
use tulip_test::benchmark_utils::SAMPLE_SIZE;
use tulip_test::criterion_logger::TimingMeasurements;
use tulip_test::database::{get_all_stock_data, init_database_data};

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

// Utility function to expand data
fn expand_close(
    pattern_open: &[f64],
    pattern_high: &[f64],
    pattern_low: &[f64],
    pattern_close: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close_vec = CLOSE.to_vec();
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    let mut open_vec = OPEN.to_vec();
    for i in 0..500 {
        if i & (i - 1) == 0 {
            close_vec.extend_from_slice(pattern_close);
            high_vec.extend_from_slice(pattern_high);
            low_vec.extend_from_slice(pattern_low);
            open_vec.extend_from_slice(pattern_open);
        }
        close_vec.extend_from_slice(&CLOSE);
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
        open_vec.extend_from_slice(&OPEN);
    }
    (open_vec, high_vec, low_vec, close_vec)
}

fn bench_candle_patterns(c: &mut Criterion) {
    let mut g = c.benchmark_group("Candle_Patterns");
    g.sample_size(SAMPLE_SIZE);
    let mut pattern_open = [87.30, 86.40, 85.40, 83.50];
    let mut pattern_high = [87.30, 86.40, 84.50, 87.95];
    let mut pattern_low = [86.30, 85.30, 83.0, 83.45];
    let mut pattern_close: [f64; 4] = [86.30, 85.30, 84.30, 87.90];

    let options = vec![7.0, 5.0, 70.0, 0.5, 3.0];
    let (mut open_vec, mut high_vec, mut low_vec, mut close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    let mut inputs = [open_vec.as_slice(), high_vec.as_slice(), low_vec.as_slice(), close_vec.as_slice()];

    if should_log_to_db() {
        // Database logging mode
        init_database_data(); // Initialize database data
        init_logging("candle_stick_indicators");
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = four_bar::bearishthreelinestrike::indicator(&inputs, &options, None)
                    .expect("Bearish Three Line Strike indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishthreelinestrike",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        // Criterion benchmark mode
        g.bench_function("bearishthreelinestrike", |b| {
            b.iter(|| {
                let result = four_bar::bearishthreelinestrike::indicator(&inputs, &options, None)
                    .expect("Bearish Three Line Strike indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.40, 87.50, 88.70];
    pattern_high = [88.30, 88.40, 88.60, 88.95];
    pattern_low = [87.30, 87.40, 87.50, 87.15];
    pattern_close = [88.30, 88.40, 88.60, 87.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];

    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = four_bar::bullishthreelinestrike::indicator(&inputs, &options, None)
                    .expect("Bullish Three Line Strike indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishthreelinestrike",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishthreelinestrike", |b| {
            b.iter(|| {
                let result = four_bar::bullishthreelinestrike::indicator(&inputs, &options, None)
                    .expect("Bullish Three Line Strike indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.40, 84.30, 85.60];
    pattern_high = [87.30, 86.40, 85.50, 85.65];
    pattern_low = [86.30, 85.30, 84.0, 83.85];
    pattern_close = [86.30, 85.30, 84.0, 83.90];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];

    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = concealingbabyswallow::indicator(&inputs, &options, None)
                    .expect("concealingbabyswallow:: indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "concealingbabyswallow",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("concealingbabyswallow", |b| {
            b.iter(|| {
                let result = concealingbabyswallow::indicator(&inputs, &options, None)
                    .expect("concealingbabyswallow:: indicator failed");
                black_box(&result);
            })
        });
    }

    let mut pattern_open = [87.68, 90.0, 89.70];
    let mut pattern_high = [90.0, 90.05, 89.80];
    let mut pattern_low = [87.40, 88.50, 87.80];
    let mut pattern_close = [89.50, 89.60, 88.0];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = twocrows::indicator(&inputs, &options, None)
                    .expect("twocrows indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "twocrows",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("twocrows", |b| {
            b.iter(|| {
                let result = twocrows::indicator(&inputs, &options, None)
                    .expect("twocrows indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.50, 87.0, 86.50];
    pattern_high = [87.55, 87.15, 86.60];
    pattern_low = [86.10, 85.90, 85.20];
    pattern_close = [86.50, 86.0, 85.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threeblackcrows::indicator(&inputs, &options, None)
                    .expect("threeblackcrows indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threeblackcrows",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threeblackcrows", |b| {
            b.iter(|| {
                let result = threeblackcrows::indicator(&inputs, &options, None)
                    .expect("threeblackcrows indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 88.00, 87.60];
    pattern_high = [88.50, 88.10, 87.60];
    pattern_low = [87.20, 87.30, 85.90];
    pattern_close = [88.30, 87.40, 86.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threeinsidedown::indicator(&inputs, &options, None)
                    .expect("threeinsidedown indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threeinsidedown",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threeinsidedown", |b| {
            b.iter(|| {
                let result = threeinsidedown::indicator(&inputs, &options, None)
                    .expect("threeinsidedown indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.40, 86.60];
    pattern_high = [88.0, 86.55, 87.10];
    pattern_low = [86.0, 86.45, 86.90];
    pattern_close = [86.30, 86.50, 87.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threeinsideup::indicator(&inputs, &options, None)
                    .expect("threeinsideup indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threeinsideup",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threeinsideup", |b| {
            b.iter(|| {
                let result = threeinsideup::indicator(&inputs, &options, None)
                    .expect("threeinsideup indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.40, 88.50, 87.60];
    pattern_high = [88.10, 88.60, 87.65];
    pattern_low = [87.30, 87.10, 84.90];
    pattern_close = [88.00, 87.20, 85.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threeoutsidedown::indicator(&inputs, &options, None)
                    .expect("threeoutsidedown indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threeoutsidedown",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threeoutsidedown", |b| {
            b.iter(|| {
                let result = threeoutsidedown::indicator(&inputs, &options, None)
                    .expect("threeoutsidedown indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [88.30, 86.20, 89.00];
    pattern_high = [88.40, 89.60, 91.85];
    pattern_low = [87.27, 86.10, 88.90];
    pattern_close = [87.28, 89.50, 91.80];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threeoutsideup::indicator(&inputs, &options, None)
                    .expect("threeoutsideup indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threeoutsideup",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threeoutsideup", |b| {
            b.iter(|| {
                let result = threeoutsideup::indicator(&inputs, &options, None)
                    .expect("threeoutsideup indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.00, 86.98, 86.0];
    pattern_high = [87.05, 87.02, 86.0];
    pattern_low = [85.0, 85.42, 85.90];
    pattern_close = [86.50, 85.45, 85.90];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threestarsinthesouth::indicator(&inputs, &options, None)
                    .expect("threestarsinthesouth indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threestarsinthesouth",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threestarsinthesouth", |b| {
            b.iter(|| {
                let result = threestarsinthesouth::indicator(&inputs, &options, None)
                    .expect("threestarsinthesouth indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [84.28, 85.25, 86.45];
    pattern_high = [85.55, 86.65, 87.55];
    pattern_low = [84.10, 85.20, 86.20];
    pattern_close = [85.50, 86.50, 87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = threewhitesoldiers::indicator(&inputs, &options, None)
                    .expect("threewhitesoldiers indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "threewhitesoldiers",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("threewhitesoldiers", |b| {
            b.iter(|| {
                let result = threewhitesoldiers::indicator(&inputs, &options, None)
                    .expect("threewhitesoldiers indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.90, 88.50];
    pattern_high = [88.35, 89.50, 89.80];
    pattern_low = [87.25, 87.70, 88.25];
    pattern_close = [88.30, 88.51, 89.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = advanceblock::indicator(&inputs, &options, None)
                    .expect("advanceblock indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "advanceblock",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("advanceblock", |b| {
            b.iter(|| {
                let result = advanceblock::indicator(&inputs, &options, None)
                    .expect("advanceblock indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 91.10, 89.70];
    pattern_high = [90.0, 91.15, 89.97];
    pattern_low = [87.10, 90.95, 88.20];
    pattern_close = [89.50, 91.10, 88.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearabandonedbaby::indicator(&inputs, &options, None)
                    .expect("bearabandonedbaby indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearabandonedbaby",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearabandonedbaby", |b| {
            b.iter(|| {
                let result = bearabandonedbaby::indicator(&inputs, &options, None)
                    .expect("bearabandonedbaby indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.64, 90.10, 87.50];
    pattern_high = [90.0, 90.15, 89.97];
    pattern_low = [87.10, 89.95, 87.20];
    pattern_close = [87.64, 90.10, 87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishtristar::indicator(&inputs, &options, None)
                    .expect("bearishtristar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishtristar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishtristar", |b| {
            b.iter(|| {
                let result = bearishtristar::indicator(&inputs, &options, None)
                    .expect("bearishtristar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.60, 86.50, 86.51];
    pattern_high = [87.65, 86.60, 86.61];
    pattern_low = [86.90, 86.40, 86.39];
    pattern_close = [87.00, 86.60, 86.59];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearsidebysidewhitelines::indicator(&inputs, &options, None)
                    .expect("bearsidebysidewhitelines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearsidebysidewhitelines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearsidebysidewhitelines", |b| {
            b.iter(|| {
                let result = bearsidebysidewhitelines::indicator(&inputs, &options, None)
                    .expect("bearsidebysidewhitelines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 85.00, 86.60];
    pattern_high = [87.20, 85.30, 87.20];
    pattern_low = [86.45, 84.50, 86.50];
    pattern_close = [86.50, 85.00, 87.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullabandonedbaby::indicator(&inputs, &options, None)
                    .expect("bullabandonedbaby indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullabandonedbaby",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullabandonedbaby", |b| {
            b.iter(|| {
                let result = bullabandonedbaby::indicator(&inputs, &options, None)
                    .expect("bullabandonedbaby indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.28, 86.90, 87.28];
    pattern_high = [90.0, 87.30, 89.97];
    pattern_low = [87.10, 86.80, 87.20];
    pattern_close = [87.28, 86.90, 87.28];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishtristar::indicator(&inputs, &options, None)
                    .expect("bullishtristar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishtristar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishtristar", |b| {
            b.iter(|| {
                let result = bullishtristar::indicator(&inputs, &options, None)
                    .expect("bullishtristar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20, 88.15, 88.13];
    pattern_high = [88.05, 88.85, 88.84];
    pattern_low = [87.15, 88.10, 88.11];
    pattern_close = [88.00, 88.80, 88.82];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullsidebysidewhitelines::indicator(&inputs, &options, None)
                    .expect("bullsidebysidewhitelines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullsidebysidewhitelines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullsidebysidewhitelines", |b| {
            b.iter(|| {
                let result = bullsidebysidewhitelines::indicator(&inputs, &options, None)
                    .expect("bullsidebysidewhitelines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.10, 86.90];
    pattern_high = [88.35, 87.20, 87.0];
    pattern_low = [87.25, 87.05, 86.40];
    pattern_close = [88.30, 87.10, 86.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = collapsingdojistar::indicator(&inputs, &options, None)
                    .expect("collapsingdojistar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "collapsingdojistar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("collapsingdojistar", |b| {
            b.iter(|| {
                let result = collapsingdojistar::indicator(&inputs, &options, None)
                    .expect("collapsingdojistar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.90, 88.80];
    pattern_high = [88.35, 89.50, 89.20];
    pattern_low = [87.25, 87.70, 88.75];
    pattern_close = [88.30, 88.90, 89.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = deliberation::indicator(&inputs, &options, None)
                    .expect("deliberation indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "deliberation",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("deliberation", |b| {
            b.iter(|| {
                let result = deliberation::indicator(&inputs, &options, None)
                    .expect("deliberation indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.60, 86.50, 86.20];
    pattern_high = [87.65, 86.60, 87.80];
    pattern_low = [86.90, 86.10, 86.15];
    pattern_close = [87.00, 86.15, 87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = downsidegapthreemethods::indicator(&inputs, &options, None)
                    .expect("downsidegapthreemethods indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "downsidegapthreemethods",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("downsidegapthreemethods", |b| {
            b.iter(|| {
                let result = downsidegapthreemethods::indicator(&inputs, &options, None)
                    .expect("downsidegapthreemethods indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.60, 86.50, 86.20];
    pattern_high = [87.65, 86.60, 86.80];
    pattern_low = [86.90, 86.10, 86.15];
    pattern_close = [87.00, 86.15, 86.70];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = downsidetasukigap::indicator(&inputs, &options, None)
                    .expect("downsidetasukigap indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "downsidetasukigap",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("downsidetasukigap", |b| {
            b.iter(|| {
                let result = downsidetasukigap::indicator(&inputs, &options, None)
                    .expect("downsidetasukigap indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 90.10, 89.70];
    pattern_high = [90.0, 90.15, 89.97];
    pattern_low = [87.10, 89.95, 88.20];
    pattern_close = [89.50, 90.10, 88.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = eveningdojistar::indicator(&inputs, &options, None)
                    .expect("eveningdojistar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "eveningdojistar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("eveningdojistar", |b| {
            b.iter(|| {
                let result = eveningdojistar::indicator(&inputs, &options, None)
                    .expect("eveningdojistar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 90.0, 89.70];
    pattern_high = [90.0, 90.15, 89.80];
    pattern_low = [87.10, 89.95, 87.20];
    pattern_close = [89.50, 90.10, 87.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = eveningstar::indicator(&inputs, &options, None)
                    .expect("eveningstar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "eveningstar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("eveningstar", |b| {
            b.iter(|| {
                let result = eveningstar::indicator(&inputs, &options, None)
                    .expect("eveningstar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.50, 86.49, 85.52];
    pattern_high = [87.55, 87.15, 85.60];
    pattern_low = [86.10, 85.40, 84.20];
    pattern_close = [86.50, 85.50, 84.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = identicalthreecrows::indicator(&inputs, &options, None)
                    .expect("identicalthreecrows indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "identicalthreecrows",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("identicalthreecrows", |b| {
            b.iter(|| {
                let result = identicalthreecrows::indicator(&inputs, &options, None)
                    .expect("identicalthreecrows indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 86.35, 86.60];
    pattern_high = [87.20, 86.55, 87.20];
    pattern_low = [86.45, 86.25, 86.50];
    pattern_close = [86.50, 86.35, 87.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = morningdojistar::indicator(&inputs, &options, None)
                    .expect("morningdojistar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "morningdojistar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("morningdojistar", |b| {
            b.iter(|| {
                let result = morningdojistar::indicator(&inputs, &options, None)
                    .expect("morningdojistar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 86.40, 86.60];
    pattern_high = [87.20, 86.55, 87.20];
    pattern_low = [86.45, 86.25, 86.50];
    pattern_close = [86.50, 86.30, 87.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = morningstar::indicator(&inputs, &options, None)
                    .expect("morningstar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "morningstar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("morningstar", |b| {
            b.iter(|| {
                let result = morningstar::indicator(&inputs, &options, None)
                    .expect("morningstar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20, 87.00, 86.40];
    pattern_high = [87.25, 87.10, 86.55];
    pattern_low = [85.50, 85.25, 86.20];
    pattern_close = [85.70, 86.50, 86.49];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = uniquethreeriverbottom::indicator(&inputs, &options, None)
                    .expect("uniquethreeriverbottom indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "uniquethreeriverbottom",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("uniquethreeriverbottom", |b| {
            b.iter(|| {
                let result = uniquethreeriverbottom::indicator(&inputs, &options, None)
                    .expect("uniquethreeriverbottom indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.68, 90.0, 90.10];
    pattern_high = [90.0, 90.05, 90.80];
    pattern_low = [87.40, 88.50, 89.20];
    pattern_close = [89.50, 89.60, 89.51];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = upsidegaptwocrows::indicator(&inputs, &options, None)
                    .expect("upsidegaptwocrows indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "upsidegaptwocrows",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("upsidegaptwocrows", |b| {
            b.iter(|| {
                let result = upsidegaptwocrows::indicator(&inputs, &options, None)
                    .expect("upsidegaptwocrows indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 89.10, 89.15];
    pattern_high = [88.30, 89.60, 89.20];
    pattern_low = [87.05, 89.00, 88.15];
    pattern_close = [88.29, 89.50, 88.20];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = upsidegapthreemethods::indicator(&inputs, &options, None)
                    .expect("upsidegapthreemethods indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "upsidegapthreemethods",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("upsidegapthreemethods", |b| {
            b.iter(|| {
                let result = upsidegapthreemethods::indicator(&inputs, &options, None)
                    .expect("upsidegapthreemethods indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.15, 89.10, 89.15];
    pattern_high = [88.30, 89.60, 86.80];
    pattern_low = [87.05, 89.00, 88.35];
    pattern_close = [88.29, 89.50, 88.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = upsidetasukigap::indicator(&inputs, &options, None)
                    .expect("upsidetasukigap indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "upsidetasukigap",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("upsidetasukigap", |b| {
            b.iter(|| {
                let result = upsidetasukigap::indicator(&inputs, &options, None)
                    .expect("upsidetasukigap indicator failed");
                black_box(&result);
            })
        });
    }

    let mut pattern_open = [86.30, 85.20];
    let mut pattern_high = [86.40, 85.55];
    let mut pattern_low = [85.0, 84.70];
    let mut pattern_close = [85.30, 84.80];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = twoblackgappingcandles::indicator(&inputs, &options, None)
                    .expect("twoblackgappingcandles indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "twoblackgappingcandles",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("twoblackgappingcandles", |b| {
            b.iter(|| {
                let result = twoblackgappingcandles::indicator(&inputs, &options, None)
                    .expect("twoblackgappingcandles indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 88.35];
    pattern_high = [88.50, 89.80];
    pattern_low = [87.20, 88.35];
    pattern_close = [88.30, 88.65];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = twocandleshootingstar::indicator(&inputs, &options, None)
                    .expect("twocandleshootingstar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "twocandleshootingstar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("twocandleshootingstar", |b| {
            b.iter(|| {
                let result = twocandleshootingstar::indicator(&inputs, &options, None)
                    .expect("twocandleshootingstar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 88.50];
    pattern_high = [88.50, 89.0];
    pattern_low = [87.25, 88.30];
    pattern_close = [88.30, 88.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishdojistar::indicator(&inputs, &options, None)
                    .expect("bearishdojistar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishdojistar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishdojistar", |b| {
            b.iter(|| {
                let result = bearishdojistar::indicator(&inputs, &options, None)
                    .expect("bearishdojistar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.40, 88.50];
    pattern_high = [88.10, 88.60];
    pattern_low = [87.30, 87.10];
    pattern_close = [88.00, 87.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishengulfing::indicator(&inputs, &options, None)
                    .expect("bearishengulfing indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishengulfing",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishengulfing", |b| {
            b.iter(|| {
                let result = bearishengulfing::indicator(&inputs, &options, None)
                    .expect("bearishengulfing indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 88.00];
    pattern_high = [88.50, 88.10];
    pattern_low = [87.20, 87.30];
    pattern_close = [88.30, 87.40];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishharami::indicator(&inputs, &options, None)
                    .expect("bearishharami indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishharami",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishharami", |b| {
            b.iter(|| {
                let result = bearishharami::indicator(&inputs, &options, None)
                    .expect("bearishharami indicator failed");
                black_box(&result);
            })
        });
    }

    // bearish harami cross
    pattern_open = [87.30, 87.50];
    pattern_high = [88.50, 88.10];
    pattern_low = [87.20, 87.35];
    pattern_close = [88.30, 87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishharamicross::indicator(&inputs, &options, None)
                    .expect("bearishharamicross indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishharamicross",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishharamicross", |b| {
            b.iter(|| {
                let result = bearishharamicross::indicator(&inputs, &options, None)
                    .expect("bearishharamicross indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 89.00];
    pattern_high = [88.50, 89.10];
    pattern_low = [87.20, 88.10];
    pattern_close = [88.30, 88.30];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishmeetinglines::indicator(&inputs, &options, None)
                    .expect("bearishmeetinglines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishmeetinglines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishmeetinglines", |b| {
            b.iter(|| {
                let result = bearishmeetinglines::indicator(&inputs, &options, None)
                    .expect("bearishmeetinglines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.30, 86.30];
    pattern_high = [88.0, 86.55];
    pattern_low = [86.0, 85.10];
    pattern_close = [87.25, 85.30];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishseparatinglines::indicator(&inputs, &options, None)
                    .expect("bearishseparatinglines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishseparatinglines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishseparatinglines", |b| {
            b.iter(|| {
                let result = bearishseparatinglines::indicator(&inputs, &options, None)
                    .expect("bearishseparatinglines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 88.20];
    pattern_high = [88.50, 88.30];
    pattern_low = [87.20, 87.10];
    pattern_close = [88.30, 87.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishtasukiline::indicator(&inputs, &options, None)
                    .expect("bearishtasukiline indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishtasukiline",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishtasukiline", |b| {
            b.iter(|| {
                let result = bearishtasukiline::indicator(&inputs, &options, None)
                    .expect("bearishtasukiline indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.50];
    pattern_high = [87.50, 86.0];
    pattern_low = [86.25, 85.30];
    pattern_close = [86.30, 85.50];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishdojistar::indicator(&inputs, &options, None)
                    .expect("bullishdojistar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishdojistar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishdojistar", |b| {
            b.iter(|| {
                let result = bullishdojistar::indicator(&inputs, &options, None)
                    .expect("bullishdojistar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [88.30, 86.20];
    pattern_high = [88.40, 89.60];
    pattern_low = [87.27, 86.10];
    pattern_close = [87.28, 89.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishengulfing::indicator(&inputs, &options, None)
                    .expect("bullishengulfing indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishengulfing",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishengulfing", |b| {
            b.iter(|| {
                let result = bullishengulfing::indicator(&inputs, &options, None)
                    .expect("bullishengulfing indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.40];
    pattern_high = [88.0, 86.55];
    pattern_low = [86.0, 86.45];
    pattern_close = [86.30, 86.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishharami::indicator(&inputs, &options, None)
                    .expect("bullishharami indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishharami",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishharami", |b| {
            b.iter(|| {
                let result = bullishharami::indicator(&inputs, &options, None)
                    .expect("bullishharami indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.50];
    pattern_high = [88.0, 86.65];
    pattern_low = [86.0, 86.40];
    pattern_close = [86.30, 86.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishharamicross::indicator(&inputs, &options, None)
                    .expect("bullishharamicross indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishharamicross",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishharamicross", |b| {
            b.iter(|| {
                let result = bullishharamicross::indicator(&inputs, &options, None)
                    .expect("bullishharamicross indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.30];
    pattern_high = [88.0, 86.55];
    pattern_low = [86.0, 85.10];
    pattern_close = [86.30, 86.30];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishmeetinglines::indicator(&inputs, &options, None)
                    .expect("bullishmeetinglines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishmeetinglines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishmeetinglines", |b| {
            b.iter(|| {
                let result = bullishmeetinglines::indicator(&inputs, &options, None)
                    .expect("bullishmeetinglines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [88.30, 88.30];
    pattern_high = [88.50, 89.10];
    pattern_low = [87.20, 88.10];
    pattern_close = [87.30, 89.0];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishseparatinglines::indicator(&inputs, &options, None)
                    .expect("bullishseparatinglines indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishseparatinglines",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishseparatinglines", |b| {
            b.iter(|| {
                let result = bullishseparatinglines::indicator(&inputs, &options, None)
                    .expect("bullishseparatinglines indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.10, 86.20];
    pattern_high = [87.15, 87.30];
    pattern_low = [86.0, 86.10];
    pattern_close = [86.10, 87.20];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishtasukiline::indicator(&inputs, &options, None)
                    .expect("bullishtasukiline indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishtasukiline",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishtasukiline", |b| {
            b.iter(|| {
                let result = bullishtasukiline::indicator(&inputs, &options, None)
                    .expect("bullishtasukiline indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 89.00];
    pattern_high = [88.50, 89.10];
    pattern_low = [87.20, 88.10];
    pattern_close = [88.30, 87.35];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = darkcloudcover::indicator(&inputs, &options, None)
                    .expect("darkcloudcover indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "darkcloudcover",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("darkcloudcover", |b| {
            b.iter(|| {
                let result = darkcloudcover::indicator(&inputs, &options, None)
                    .expect("darkcloudcover indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.40];
    pattern_high = [88.50, 88.10];
    pattern_low = [87.20, 87.30];
    pattern_close = [88.30, 88.00];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = descendinghawk::indicator(&inputs, &options, None)
                    .expect("descendinghawk indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "descendinghawk",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("descendinghawk", |b| {
            b.iter(|| {
                let result = descendinghawk::indicator(&inputs, &options, None)
                    .expect("descendinghawk indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.80];
    pattern_high = [87.50, 86.00];
    pattern_low = [86.20, 85.30];
    pattern_close = [86.30, 85.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = fallingwindow::indicator(&inputs, &options, None)
                    .expect("fallingwindow indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "fallingwindow",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("fallingwindow", |b| {
            b.iter(|| {
                let result = fallingwindow::indicator(&inputs, &options, None)
                    .expect("fallingwindow indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.50];
    pattern_high = [88.0, 86.55];
    pattern_low = [86.0, 86.35];
    pattern_close = [86.30, 86.40];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = homingpigeon::indicator(&inputs, &options, None)
                    .expect("homingpigeon indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "homingpigeon",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("homingpigeon", |b| {
            b.iter(|| {
                let result = homingpigeon::indicator(&inputs, &options, None)
                    .expect("homingpigeon indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.00];
    pattern_high = [87.50, 86.50];
    pattern_low = [86.20, 86.30];
    pattern_close = [86.30, 86.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    inneck::indicator(&inputs, &options, None).expect("inneck indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "inneck",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("inneck", |b| {
            b.iter(|| {
                let result =
                    inneck::indicator(&inputs, &options, None).expect("inneck indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.00];
    pattern_high = [87.50, 87.60];
    pattern_low = [86.20, 86.0];
    pattern_close = [86.30, 86.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = invertedhammer::indicator(&inputs, &options, None)
                    .expect("invertedhammer indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "invertedhammer",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("invertedhammer", |b| {
            b.iter(|| {
                let result = invertedhammer::indicator(&inputs, &options, None)
                    .expect("invertedhammer indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.20, 86.19];
    pattern_high = [87.30, 86.19];
    pattern_low = [86.20, 85.20];
    pattern_close = [87.30, 85.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = kickingdown::indicator(&inputs, &options, None)
                    .expect("kickingdown indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "kickingdown",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("kickingdown", |b| {
            b.iter(|| {
                let result = kickingdown::indicator(&inputs, &options, None)
                    .expect("kickingdown indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.19, 86.20];
    pattern_high = [86.19, 87.30];
    pattern_low = [85.20, 86.20];
    pattern_close = [85.20, 87.30];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = kickingup::indicator(&inputs, &options, None)
                    .expect("kickingup indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "kickingup",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("kickingup", |b| {
            b.iter(|| {
                let result = kickingup::indicator(&inputs, &options, None)
                    .expect("kickingup indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20, 87.50];
    pattern_high = [88.0, 88.0];
    pattern_low = [87.0, 87.20];
    pattern_close = [88.00, 88.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = matchinghigh::indicator(&inputs, &options, None)
                    .expect("matchinghigh indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "matchinghigh",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("matchinghigh", |b| {
            b.iter(|| {
                let result = matchinghigh::indicator(&inputs, &options, None)
                    .expect("matchinghigh indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20, 87.0];
    pattern_high = [87.30, 87.20];
    pattern_low = [86.0, 86.0];
    pattern_close = [86.0, 86.0];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = matchinglow::indicator(&inputs, &options, None)
                    .expect("matchinglow indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "matchinglow",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("matchinglow", |b| {
            b.iter(|| {
                let result = matchinglow::indicator(&inputs, &options, None)
                    .expect("matchinglow indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 86.00];
    pattern_high = [87.50, 86.50];
    pattern_low = [86.20, 86.10];
    pattern_close = [86.30, 86.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    onneck::indicator(&inputs, &options, None).expect("onneck indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "onneck",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("onneck", |b| {
            b.iter(|| {
                let result =
                    onneck::indicator(&inputs, &options, None).expect("onneck indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.50];
    pattern_high = [87.50, 86.80];
    pattern_low = [86.20, 85.50];
    pattern_close = [86.30, 86.75];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = thrusting::indicator(&inputs, &options, None)
                    .expect("thrusting indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "thrusting",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("thrusting", |b| {
            b.iter(|| {
                let result = thrusting::indicator(&inputs, &options, None)
                    .expect("thrusting indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20, 86.15];
    pattern_high = [87.25, 87.20];
    pattern_low = [86.15, 86.10];
    pattern_close = [86.20, 87.15];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = piercing::indicator(&inputs, &options, None)
                    .expect("piercing indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "piercing",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("piercing", |b| {
            b.iter(|| {
                let result = piercing::indicator(&inputs, &options, None)
                    .expect("piercing indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [85.80, 87.30];
    pattern_high = [86.00, 87.50];
    pattern_low = [86.00, 86.20];
    pattern_close = [85.40, 86.30];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = risingwindow::indicator(&inputs, &options, None)
                    .expect("risingwindow indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "risingwindow",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("risingwindow", |b| {
            b.iter(|| {
                let result = risingwindow::indicator(&inputs, &options, None)
                    .expect("risingwindow indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.80];
    pattern_high = [87.50, 86.00];
    pattern_low = [86.20, 85.30];
    pattern_close = [86.30, 85.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = turndown::indicator(&inputs, &options, None)
                    .expect("turndown indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "turndown",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("turndown", |b| {
            b.iter(|| {
                let result = turndown::indicator(&inputs, &options, None)
                    .expect("turndown indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [85.80, 87.30];
    pattern_high = [86.00, 87.55];
    pattern_low = [85.35, 87.20];
    pattern_close = [85.40, 87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    turnup::indicator(&inputs, &options, None).expect("turnup indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "turnup",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("turnup", |b| {
            b.iter(|| {
                let result =
                    turnup::indicator(&inputs, &options, None).expect("turnup indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 85.80];
    pattern_high = [87.50, 86.00];
    pattern_low = [86.20, 86.20];
    pattern_close = [86.30, 86.40];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = tweezersbottom::indicator(&inputs, &options, None)
                    .expect("tweezersbottom indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "tweezersbottom ",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("tweezersbottom ", |b| {
            b.iter(|| {
                let result = tweezersbottom::indicator(&inputs, &options, None)
                    .expect("tweezersbottom indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.30, 87.80];
    pattern_high = [88.50, 88.50];
    pattern_low = [87.20, 87.20];
    pattern_close = [88.30, 87.70];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = tweezerstop::indicator(&inputs, &options, None)
                    .expect("tweezerstop indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "tweezerstop ",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("tweezerstop ", |b| {
            b.iter(|| {
                let result = tweezerstop::indicator(&inputs, &options, None)
                    .expect("tweezerstop indicator failed");
                black_box(&result);
            })
        });
    }

    let mut pattern_open = [88.50];
    let mut pattern_high = [88.50];
    let mut pattern_low = [87.25];
    let mut pattern_close = [87.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishbelthold::indicator(&inputs, &options, None)
                    .expect("bearishbelthold indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishbelthold",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishbelthold", |b| {
            b.iter(|| {
                let result = bearishbelthold::indicator(&inputs, &options, None)
                    .expect("bearishbelthold indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [90.90];
    pattern_high = [90.95];
    pattern_low = [85.65];
    pattern_close = [85.70];
    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bearishstrongline::indicator(&inputs, &options, None)
                    .expect("bearishstrongline indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bearishstrongline",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bearishstrongline", |b| {
            b.iter(|| {
                let result = bearishstrongline::indicator(&inputs, &options, None)
                    .expect("bearishstrongline indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [85.75];
    pattern_high = [87.49];
    pattern_low = [85.75];
    pattern_close = [87.25];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = bullishbelthold::indicator(&inputs, &options, None)
                    .expect("bullishbelthold indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "bullishbelthold",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("bullishbelthold", |b| {
            b.iter(|| {
                let result = bullishbelthold::indicator(&inputs, &options, None)
                    .expect("bullishbelthold indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.50];
    pattern_high = [86.50];
    pattern_low = [85.25];
    pattern_close = [86.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = gappingdowndoji::indicator(&inputs, &options, None)
                    .expect("gappingdowndoji indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "gappingdowndoji",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("gappingdowndoji", |b| {
            b.iter(|| {
                let result = gappingdowndoji::indicator(&inputs, &options, None)
                    .expect("gappingdowndoji indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [88.50];
    pattern_high = [88.50];
    pattern_low = [88.25];
    pattern_close = [88.50];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = gappingupdoji::indicator(&inputs, &options, None)
                    .expect("gappingupdoji indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "gappingupdoji",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("gappingupdoji", |b| {
            b.iter(|| {
                let result = gappingupdoji::indicator(&inputs, &options, None)
                    .expect("gappingupdoji indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.90];
    pattern_high = [87.0];
    pattern_low = [85.75];
    pattern_close = [86.60];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result =
                    hammer::indicator(&inputs, &options, None).expect("hammer indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "hammer",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("hammer", |b| {
            b.iter(|| {
                let result =
                    hammer::indicator(&inputs, &options, None).expect("hammer indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.35];
    pattern_high = [87.35];
    pattern_low = [85.15];
    pattern_close = [87.30];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = hangingman::indicator(&inputs, &options, None)
                    .expect("hangingman indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "hangingman",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("hangingman", |b| {
            b.iter(|| {
                let result = hangingman::indicator(&inputs, &options, None)
                    .expect("hangingman indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.65];
    pattern_high = [87.70];
    pattern_low = [85.15];
    pattern_close = [87.65];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = northerndoji::indicator(&inputs, &options, None)
                    .expect("northerndoji indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "northerndoji",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("northerndoji", |b| {
            b.iter(|| {
                let result = northerndoji::indicator(&inputs, &options, None)
                    .expect("northerndoji indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.40];
    pattern_high = [88.90];
    pattern_low = [87.25];
    pattern_close = [87.30];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = onecandleshootingstar::indicator(&inputs, &options, None)
                    .expect("onecandleshootingstar indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "onecandleshootingstar",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("onecandleshootingstar", |b| {
            b.iter(|| {
                let result = onecandleshootingstar::indicator(&inputs, &options, None)
                    .expect("onecandleshootingstar indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [87.20];
    pattern_high = [87.25];
    pattern_low = [85.15];
    pattern_close = [87.20];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = southerndoji::indicator(&inputs, &options, None)
                    .expect("southerndoji indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "southerndoji",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("southerndoji", |b| {
            b.iter(|| {
                let result = southerndoji::indicator(&inputs, &options, None)
                    .expect("southerndoji indicator failed");
                black_box(&result);
            })
        });
    }

    pattern_open = [86.90];
    pattern_high = [87.0];
    pattern_low = [85.65];
    pattern_close = [86.60];

    (open_vec, high_vec, low_vec, close_vec) =
        expand_close(&pattern_open, &pattern_high, &pattern_low, &pattern_close);
    inputs = [&open_vec, &high_vec, &low_vec, &close_vec];
    if should_log_to_db() {
        let mut timing = TimingMeasurements::new();
        timing.measure(
            || {
                let result = takuriline::indicator(&inputs, &options, None)
                    .expect("takuriline indicator failed");
                black_box(&result);
            },
            1000,
        );

        log_timing_result(
            "takuriline",
            "Rust",
            &options,
            inputs[0].len(),
            &timing,
            Some("PATTERN_DATA"),
        );
    } else {
        g.bench_function("takuriline", |b| {
            b.iter(|| {
                let result = takuriline::indicator(&inputs, &options, None)
                    .expect("takuriline indicator failed");
                black_box(&result);
            })
        });
    }
    g.finish();
}

/// Benchmark candlestick patterns with database stocks
fn bench_database_candle_patterns(c: &mut Criterion) {
    init_database_data();
    init_logging("candle_stick_indicators");
    if let Some(stock_data) = get_all_stock_data() {
        let options = vec![7.0, 5.0, 70.0, 0.5, 3.0];

        for (stock_name, eod_data) in stock_data {
            // Extract OHLC arrays from EodData
            let open: Vec<f64> = eod_data.iter().map(|d| d.open).collect();
            let high: Vec<f64> = eod_data.iter().map(|d| d.high).collect();
            let low: Vec<f64> = eod_data.iter().map(|d| d.low).collect();
            let close: Vec<f64> = eod_data.iter().map(|d| d.close).collect();

            let inputs = [open.as_slice(), &high.as_slice(), &low.as_slice(), &close.as_slice()];

            // Test all candlestick patterns with database data

            // Four-bar patterns
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            four_bar::bearishthreelinestrike::indicator(&inputs, &options, None)
                                .expect("bearishthreelinestrike indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishthreelinestrike",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishthreelinestrike - {}", stock_name), |b| {
                    b.iter(|| {
                        let result =
                            four_bar::bearishthreelinestrike::indicator(&inputs, &options, None)
                                .expect("bearishthreelinestrike indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result =
                            four_bar::bullishthreelinestrike::indicator(&inputs, &options, None)
                                .expect("bullishthreelinestrike indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishthreelinestrike",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishthreelinestrike - {}", stock_name), |b| {
                    b.iter(|| {
                        let result =
                            four_bar::bullishthreelinestrike::indicator(&inputs, &options, None)
                                .expect("bullishthreelinestrike indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = concealingbabyswallow::indicator(&inputs, &options, None)
                            .expect("concealingbabyswallow indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "concealingbabyswallow",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("concealingbabyswallow - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = concealingbabyswallow::indicator(&inputs, &options, None)
                            .expect("concealingbabyswallow indicator failed");
                        black_box(&result);
                    })
                });
            }

            // Three-bar patterns
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = twocrows::indicator(&inputs, &options, None)
                            .expect("twocrows indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "twocrows",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("twocrows - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = twocrows::indicator(&inputs, &options, None)
                            .expect("twocrows indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threeblackcrows::indicator(&inputs, &options, None)
                            .expect("threeblackcrows indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threeblackcrows",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threeblackcrows - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threeblackcrows::indicator(&inputs, &options, None)
                            .expect("threeblackcrows indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threeinsidedown::indicator(&inputs, &options, None)
                            .expect("threeinsidedown indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threeinsidedown",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threeinsidedown - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threeinsidedown::indicator(&inputs, &options, None)
                            .expect("threeinsidedown indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threeinsideup::indicator(&inputs, &options, None)
                            .expect("threeinsideup indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threeinsideup",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threeinsideup - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threeinsideup::indicator(&inputs, &options, None)
                            .expect("threeinsideup indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threeoutsidedown::indicator(&inputs, &options, None)
                            .expect("threeoutsidedown indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threeoutsidedown",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threeoutsidedown - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threeoutsidedown::indicator(&inputs, &options, None)
                            .expect("threeoutsidedown indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threeoutsideup::indicator(&inputs, &options, None)
                            .expect("threeoutsideup indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threeoutsideup",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threeoutsideup - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threeoutsideup::indicator(&inputs, &options, None)
                            .expect("threeoutsideup indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threestarsinthesouth::indicator(&inputs, &options, None)
                            .expect("threestarsinthesouth indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threestarsinthesouth",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threestarsinthesouth - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threestarsinthesouth::indicator(&inputs, &options, None)
                            .expect("threestarsinthesouth indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = threewhitesoldiers::indicator(&inputs, &options, None)
                            .expect("threewhitesoldiers indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "threewhitesoldiers",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("threewhitesoldiers - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = threewhitesoldiers::indicator(&inputs, &options, None)
                            .expect("threewhitesoldiers indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = advanceblock::indicator(&inputs, &options, None)
                            .expect("advanceblock indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "advanceblock",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("advanceblock - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = advanceblock::indicator(&inputs, &options, None)
                            .expect("advanceblock indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearabandonedbaby::indicator(&inputs, &options, None)
                            .expect("bearabandonedbaby indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearabandonedbaby",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearabandonedbaby - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearabandonedbaby::indicator(&inputs, &options, None)
                            .expect("bearabandonedbaby indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishtristar::indicator(&inputs, &options, None)
                            .expect("bearishtristar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishtristar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishtristar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishtristar::indicator(&inputs, &options, None)
                            .expect("bearishtristar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearsidebysidewhitelines::indicator(&inputs, &options, None)
                            .expect("bearsidebysidewhitelines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearsidebysidewhitelines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearsidebysidewhitelines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearsidebysidewhitelines::indicator(&inputs, &options, None)
                            .expect("bearsidebysidewhitelines indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullabandonedbaby::indicator(&inputs, &options, None)
                            .expect("bullabandonedbaby indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullabandonedbaby",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullabandonedbaby - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullabandonedbaby::indicator(&inputs, &options, None)
                            .expect("bullabandonedbaby indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishtristar::indicator(&inputs, &options, None)
                            .expect("bullishtristar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishtristar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishtristar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishtristar::indicator(&inputs, &options, None)
                            .expect("bullishtristar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullsidebysidewhitelines::indicator(&inputs, &options, None)
                            .expect("bullsidebysidewhitelines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullsidebysidewhitelines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullsidebysidewhitelines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullsidebysidewhitelines::indicator(&inputs, &options, None)
                            .expect("bullsidebysidewhitelines indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = collapsingdojistar::indicator(&inputs, &options, None)
                            .expect("collapsingdojistar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "collapsingdojistar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("collapsingdojistar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = collapsingdojistar::indicator(&inputs, &options, None)
                            .expect("collapsingdojistar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = deliberation::indicator(&inputs, &options, None)
                            .expect("deliberation indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "deliberation",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("deliberation - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = deliberation::indicator(&inputs, &options, None)
                            .expect("deliberation indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = downsidegapthreemethods::indicator(&inputs, &options, None)
                            .expect("downsidegapthreemethods indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "downsidegapthreemethods",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("downsidegapthreemethods - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = downsidegapthreemethods::indicator(&inputs, &options, None)
                            .expect("downsidegapthreemethods indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = downsidetasukigap::indicator(&inputs, &options, None)
                            .expect("downsidetasukigap indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "downsidetasukigap",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("downsidetasukigap - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = downsidetasukigap::indicator(&inputs, &options, None)
                            .expect("downsidetasukigap indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = eveningdojistar::indicator(&inputs, &options, None)
                            .expect("eveningdojistar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "eveningdojistar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("eveningdojistar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = eveningdojistar::indicator(&inputs, &options, None)
                            .expect("eveningdojistar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = eveningstar::indicator(&inputs, &options, None)
                            .expect("eveningstar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "eveningstar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("eveningstar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = eveningstar::indicator(&inputs, &options, None)
                            .expect("eveningstar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = identicalthreecrows::indicator(&inputs, &options, None)
                            .expect("identicalthreecrows indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "identicalthreecrows",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("identicalthreecrows - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = identicalthreecrows::indicator(&inputs, &options, None)
                            .expect("identicalthreecrows indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = morningdojistar::indicator(&inputs, &options, None)
                            .expect("morningdojistar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "morningdojistar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("morningdojistar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = morningdojistar::indicator(&inputs, &options, None)
                            .expect("morningdojistar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = morningstar::indicator(&inputs, &options, None)
                            .expect("morningstar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "morningstar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("morningstar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = morningstar::indicator(&inputs, &options, None)
                            .expect("morningstar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = uniquethreeriverbottom::indicator(&inputs, &options, None)
                            .expect("uniquethreeriverbottom indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "uniquethreeriverbottom",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("uniquethreeriverbottom - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = uniquethreeriverbottom::indicator(&inputs, &options, None)
                            .expect("uniquethreeriverbottom indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = upsidegaptwocrows::indicator(&inputs, &options, None)
                            .expect("upsidegaptwocrows indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "upsidegaptwocrows",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("upsidegaptwocrows - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = upsidegaptwocrows::indicator(&inputs, &options, None)
                            .expect("upsidegaptwocrows indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = upsidegapthreemethods::indicator(&inputs, &options, None)
                            .expect("upsidegapthreemethods indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "upsidegapthreemethods",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("upsidegapthreemethods - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = upsidegapthreemethods::indicator(&inputs, &options, None)
                            .expect("upsidegapthreemethods indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = upsidetasukigap::indicator(&inputs, &options, None)
                            .expect("upsidetasukigap indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "upsidetasukigap",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("upsidetasukigap - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = upsidetasukigap::indicator(&inputs, &options, None)
                            .expect("upsidetasukigap indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = twoblackgappingcandles::indicator(&inputs, &options, None)
                            .expect("twoblackgappingcandles indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "twoblackgappingcandles",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("twoblackgappingcandles - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = twoblackgappingcandles::indicator(&inputs, &options, None)
                            .expect("twoblackgappingcandles indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = twocandleshootingstar::indicator(&inputs, &options, None)
                            .expect("twocandleshootingstar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "twocandleshootingstar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("twocandleshootingstar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = twocandleshootingstar::indicator(&inputs, &options, None)
                            .expect("twocandleshootingstar indicator failed");
                        black_box(&result);
                    })
                });
            }

            // Two-bar patterns
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishdojistar::indicator(&inputs, &options, None)
                            .expect("bearishdojistar indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishdojistar",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishdojistar - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishdojistar::indicator(&inputs, &options, None)
                            .expect("bearishdojistar indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishengulfing::indicator(&inputs, &options, None)
                            .expect("bearishengulfing indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishengulfing",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishengulfing - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishengulfing::indicator(&inputs, &options, None)
                            .expect("bearishengulfing indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishharami::indicator(&inputs, &options, None)
                            .expect("bearishharami indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishharami",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishharami - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishharami::indicator(&inputs, &options, None)
                            .expect("bearishharami indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishharamicross::indicator(&inputs, &options, None)
                            .expect("bearishharamicross indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishharamicross",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishharamicross - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishharamicross::indicator(&inputs, &options, None)
                            .expect("bearishharamicross indicator failed");
                        black_box(&result);
                    })
                });
            }

            // Additional two-bar patterns that still need conversion
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishmeetinglines::indicator(&inputs, &options, None)
                            .expect("bearishmeetinglines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishmeetinglines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishmeetinglines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishmeetinglines::indicator(&inputs, &options, None)
                            .expect("bearishmeetinglines indicator failed");
                        black_box(&result);
                    })
                });
            }

            // Add all missing indicators that only have PATTERN_DATA

            // Missing two-bar patterns
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishseparatinglines::indicator(&inputs, &options, None)
                            .expect("bearishseparatinglines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishseparatinglines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishseparatinglines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishseparatinglines::indicator(&inputs, &options, None)
                            .expect("bearishseparatinglines indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bearishtasukiline::indicator(&inputs, &options, None)
                            .expect("bearishtasukiline indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bearishtasukiline",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bearishtasukiline - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bearishtasukiline::indicator(&inputs, &options, None)
                            .expect("bearishtasukiline indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishengulfing::indicator(&inputs, &options, None)
                            .expect("bullishengulfing indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishengulfing",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishengulfing - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishengulfing::indicator(&inputs, &options, None)
                            .expect("bullishengulfing indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishharami::indicator(&inputs, &options, None)
                            .expect("bullishharami indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishharami",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishharami - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishharami::indicator(&inputs, &options, None)
                            .expect("bullishharami indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishmeetinglines::indicator(&inputs, &options, None)
                            .expect("bullishmeetinglines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishmeetinglines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishmeetinglines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishmeetinglines::indicator(&inputs, &options, None)
                            .expect("bullishmeetinglines indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishseparatinglines::indicator(&inputs, &options, None)
                            .expect("bullishseparatinglines indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishseparatinglines",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishseparatinglines - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishseparatinglines::indicator(&inputs, &options, None)
                            .expect("bullishseparatinglines indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = bullishtasukiline::indicator(&inputs, &options, None)
                            .expect("bullishtasukiline indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "bullishtasukiline",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("bullishtasukiline - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = bullishtasukiline::indicator(&inputs, &options, None)
                            .expect("bullishtasukiline indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = darkcloudcover::indicator(&inputs, &options, None)
                            .expect("darkcloudcover indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "darkcloudcover",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("darkcloudcover - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = darkcloudcover::indicator(&inputs, &options, None)
                            .expect("darkcloudcover indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = descendinghawk::indicator(&inputs, &options, None)
                            .expect("descendinghawk indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "descendinghawk",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("descendinghawk - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = descendinghawk::indicator(&inputs, &options, None)
                            .expect("descendinghawk indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = fallingwindow::indicator(&inputs, &options, None)
                            .expect("fallingwindow indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "fallingwindow",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("fallingwindow - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = fallingwindow::indicator(&inputs, &options, None)
                            .expect("fallingwindow indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = gappingdowndoji::indicator(&inputs, &options, None)
                            .expect("gappingdowndoji indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "gappingdowndoji",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("gappingdowndoji - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = gappingdowndoji::indicator(&inputs, &options, None)
                            .expect("gappingdowndoji indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = gappingupdoji::indicator(&inputs, &options, None)
                            .expect("gappingupdoji indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "gappingupdoji",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("gappingupdoji - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = gappingupdoji::indicator(&inputs, &options, None)
                            .expect("gappingupdoji indicator failed");
                        black_box(&result);
                    })
                });
            }

            // Single-bar patterns
            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = hangingman::indicator(&inputs, &options, None)
                            .expect("hangingman indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "hangingman",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("hangingman - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = hangingman::indicator(&inputs, &options, None)
                            .expect("hangingman indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = homingpigeon::indicator(&inputs, &options, None)
                            .expect("homingpigeon indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "homingpigeon",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("homingpigeon - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = homingpigeon::indicator(&inputs, &options, None)
                            .expect("homingpigeon indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = inneck::indicator(&inputs, &options, None)
                            .expect("inneck indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "inneck",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("inneck - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = inneck::indicator(&inputs, &options, None)
                            .expect("inneck indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = invertedhammer::indicator(&inputs, &options, None)
                            .expect("invertedhammer indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "invertedhammer",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("invertedhammer - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = invertedhammer::indicator(&inputs, &options, None)
                            .expect("invertedhammer indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = kickingup::indicator(&inputs, &options, None)
                            .expect("kickingup indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "kickingup",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("kickingup - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = kickingup::indicator(&inputs, &options, None)
                            .expect("kickingup indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = matchinghigh::indicator(&inputs, &options, None)
                            .expect("matchinghigh indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "matchinghigh",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("matchinghigh - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = matchinghigh::indicator(&inputs, &options, None)
                            .expect("matchinghigh indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = matchinglow::indicator(&inputs, &options, None)
                            .expect("matchinglow indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "matchinglow",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("matchinglow - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = matchinglow::indicator(&inputs, &options, None)
                            .expect("matchinglow indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = onneck::indicator(&inputs, &options, None)
                            .expect("onneck indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "onneck",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("onneck - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = onneck::indicator(&inputs, &options, None)
                            .expect("onneck indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = piercing::indicator(&inputs, &options, None)
                            .expect("piercing indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "piercing",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("piercing - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = piercing::indicator(&inputs, &options, None)
                            .expect("piercing indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = risingwindow::indicator(&inputs, &options, None)
                            .expect("risingwindow indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "risingwindow",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("risingwindow - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = risingwindow::indicator(&inputs, &options, None)
                            .expect("risingwindow indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = takuriline::indicator(&inputs, &options, None)
                            .expect("takuriline indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "takuriline",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("takuriline - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = takuriline::indicator(&inputs, &options, None)
                            .expect("takuriline indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = thrusting::indicator(&inputs, &options, None)
                            .expect("thrusting indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "thrusting",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("thrusting - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = thrusting::indicator(&inputs, &options, None)
                            .expect("thrusting indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = turndown::indicator(&inputs, &options, None)
                            .expect("turndown indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "turndown",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("turndown - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = turndown::indicator(&inputs, &options, None)
                            .expect("turndown indicator failed");
                        black_box(&result);
                    })
                });
            }

            if should_log_to_db() {
                let mut timing = TimingMeasurements::new();
                timing.measure(
                    || {
                        let result = turnup::indicator(&inputs, &options, None)
                            .expect("turnup indicator failed");
                        black_box(&result);
                    },
                    1000,
                );

                log_timing_result(
                    "turnup",
                    "Rust",
                    &options,
                    inputs[0].len(),
                    &timing,
                    Some(&stock_name),
                );
            } else {
                c.bench_function(&format!("turnup - {}", stock_name), |b| {
                    b.iter(|| {
                        let result = turnup::indicator(&inputs, &options, None)
                            .expect("turnup indicator failed");
                        black_box(&result);
                    })
                });
            }
        }
    }
}

criterion_group!(
    benches,
    bench_candle_patterns,
    bench_database_candle_patterns
);
criterion_main!(benches);
*/