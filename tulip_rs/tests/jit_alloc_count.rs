//! Allocation-count guard for the streaming `resume`/`tick` paths (review
//! §3.2 / Handover "the tick floor is allocation, not the loop").
//!
//! This lives in its OWN integration-test binary on purpose: the counting
//! `#[global_allocator]` is process-wide, so hosting it inside the `lib` unit
//! tests forced EVERY other cse test's allocations through it — and the TLS
//! counter touched during parallel teardown SIGSEGV'd the whole `--lib` suite
//! nondeterministically (reproduced on clean HEAD, 32 cores). A separate
//! binary scopes the allocator to exactly this measurement.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

struct Counter;

unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // `try_with`: a global allocator must never panic, including during
        // thread teardown when the TLS slot may already be gone.
        let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: Counter = Counter;

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicators::{apo::Apo, macd::Macd, ppo::Ppo};

const APO: [f64; 2] = [12.0, 26.0];
const MACD: [f64; 3] = [12.0, 26.0, 9.0];
const PPO: [f64; 2] = [12.0, 26.0];

fn series(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
        .collect()
}

#[test]
fn jit_resume_allocations() {
    let close = series(300);
    let warm: [&[f64]; 1] = [close.as_slice()];
    let plan = Request::new()
        .add(Apo::recipe(&APO), None)
        .add(Ppo::recipe(&PPO), None)
        .add(Macd::recipe(&MACD), None)
        .build();
    let (res, qs) = plan.run(&warm);
    black_box(res.outputs[0][0].len() + qs.next_bar() as usize);

    let one = [1.0f64; 1];
    let nb: [&[f64]; 1] = [one.as_slice()];
    let n = black_box(200);
    ALLOCS.with(|a| a.set(0));
    for _ in 0..n {
        let (_r, q2) = plan.resume(qs.clone(), &nb);
        black_box(q2.next_bar());
    }
    let per = ALLOCS.with(|a| a.get()) as f64 / n as f64;
    println!("allocs per 1-bar resume: {per:.1}");
    assert!(per < 16.0, "resume touched {per} allocations");
}
