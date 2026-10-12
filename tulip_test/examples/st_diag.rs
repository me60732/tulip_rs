//! natr+supertrend economics: split the fused row into fixed cost (arena
//! alloc, RowRef vec, warm phase) vs per-bar slope, and price the allocation
//! directly. Series-length sweep -> where the 15.3 µs lives.
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::time::Instant;
use tulip_rs::cse::{erase_request, try_run, Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_test::database::{get_all_stock_data, init_database_data, EodData};

fn cols(sym: usize, f: impl Fn(&EodData) -> f64) -> Vec<f64> {
    init_database_data();
    get_all_stock_data().unwrap()[sym]
        .1
        .iter()
        .map(|d| f(d))
        .collect()
}

fn time(label: &str, iters: usize, mut f: impl FnMut()) -> f64 {
    for _ in 0..3 {
        f();
    }
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / iters as f64;
    println!("{label:46} {us:8.2} µs");
    us
}

fn main() {
    let hi = cols(0, |d| d.high);
    let lo = cols(0, |d| d.low);
    let cl = cols(0, |d| d.close);
    let n = hi.len();
    println!("bars = {n}");
    let req = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let (_a, sig, vals, _o) = erase_request(&req).unwrap();

    // fused kernel at four lengths: slope = per-bar, intercept = fixed cost
    let mut fixed_at_0 = None;
    let mut prev = None;
    for m in [64usize, 1024, 4096, n] {
        let three: [&[f64]; 3] = [&hi[..m], &lo[..m], &cl[..m]];
        let us = time(
            &format!("generated_runonly @ m={m}"),
            (30_000_000 / m / 3).clamp(30, 2000),
            || {
                std::hint::black_box(try_run(&sig, &vals, &three).unwrap()[0].len());
            },
        );
        if let Some((pm, pus)) = prev {
            let slope = (us - pus) / (m - pm) as f64;
            let intercept = us - slope * m as f64;
            println!(
                "     -> {:.2} ns/bar  (vs {}..{m}), fixed ≈ {intercept:.2} µs",
                slope * 1000.0,
                pm
            );
            if pm >= 4096 {
                fixed_at_0 = Some(intercept);
            }
        }
        prev = Some((m, us));
    }
    let _ = fixed_at_0;

    // hand loops at full length for the work-floor comparison
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    time("hand natr", 300, || {
        std::hint::black_box(Natr::indicator(&three, &[14.0], None).unwrap().0[0].len());
    });
    time("hand supertrend", 300, || {
        std::hint::black_box(SuperTrend::indicator(&three, &[14.0, 3.0], None).unwrap().0[0].len());
    });

    // the arena allocation itself (one Arc slice ~106KB uninit, touched like
    // the kernel does, then dropped)
    let total = 2 * (n - 14);
    time("arena alloc+get_mut+drop (2 rows)", 3000, || {
        let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
        let p = Arc::get_mut(&mut arc).unwrap().as_mut_ptr();
        unsafe { *p = MaybeUninit::new(1.0) };
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        std::hint::black_box(&arena);
    });
    // one 53KB uninit Vec for scale
    time("one uninit Vec(n) alloc+drop", 4000, || {
        let mut v: Vec<f64> = Vec::with_capacity(n);
        unsafe { v.set_len(n) };
        let p = v.as_mut_ptr();
        unsafe { *p = 2.0 };
        std::hint::black_box(v);
    });
}
