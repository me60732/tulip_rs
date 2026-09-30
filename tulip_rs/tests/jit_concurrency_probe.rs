//! Probe: is concurrent `JitPlan` build+run safe, or is the lib-test SIGSEGV a
//! real library concurrency bug? Distinct from the flaky full-suite teardown —
//! this fails loudly + repeatably if `build()`/`advance()` race.

use std::sync::{Arc, Barrier};
use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicators::{apo::Apo, ema::Ema, macd::Macd, ppo::Ppo};

fn series(n: usize, seed: f64) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64 * 0.7 + seed).sin() * 10.0 + 100.0))
        .collect()
}

/// N threads each BUILD a distinct plan concurrently, then run it. Exercises
/// the compile path under contention (the suite crashes at teardown right
/// after builds, so this is the prime suspect).
#[test]
fn concurrent_distinct_builds_and_runs() {
    let n_threads = 16;
    let bar = Arc::new(Barrier::new(n_threads));
    std::thread::scope(|s| {
        for t in 0..n_threads {
            let bar = Arc::clone(&bar);
            s.spawn(move || {
                let close = series(400, t as f64);
                let inp: [&[f64]; 1] = [close.as_slice()];
                let req = match t % 4 {
                    0 => Request::new().add(Apo::recipe(&[12.0, 26.0]), None),
                    1 => Request::new().add(Ppo::recipe(&[12.0, 26.0]), None),
                    2 => Request::new().add(Macd::recipe(&[12.0, 26.0, 9.0]), None),
                    _ => Request::new().add(Ema::recipe(&[9.0]), None),
                };
                let plan = req.build(); // concurrent compile
                bar.wait(); // all build, then run together
                let (res, _st) = plan.run(&inp);
                assert!(!res.outputs[0][0].is_empty());
            });
        }
    });
}

/// One shared `Arc<JitPlan>`, many threads `advance` it with caller-owned
/// states — the "host-threaded assets" model (§3.3). Must be race-free for
/// that story to hold.
#[test]
fn shared_plan_threaded_runs() {
    let plan = Arc::new(
        Request::new()
            .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
            .build(),
    );
    let n_threads = 16;
    std::thread::scope(|s| {
        for t in 0..n_threads {
            let plan = Arc::clone(&plan);
            s.spawn(move || {
                let close = series(500, t as f64 * 1.7);
                let inp: [&[f64]; 1] = [close.as_slice()];
                let (res, _st) = plan.run(&inp);
                let n = res.outputs[0][0].len();
                assert!(n > 0 && n < close.len());
            });
        }
    });
}
