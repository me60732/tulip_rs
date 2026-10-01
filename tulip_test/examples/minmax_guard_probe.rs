//! E9 crash hunt: run every roll plan/split sequence against price series that
//! live in their OWN mapping with PROT_NONE guard pages on BOTH sides, so any
//! out-of-bounds read faults deterministically at the site that crossed, instead
//! of landing in innocent heap (which is why `cargo test --lib cse::` crashes
//! 6/6 while a plain standalone repro survives 400 hammers).
//!
//! The two placements matter separately:
//!   · head-flush   → data begins at the readable region, so a read of a bar
//!                    BEFORE the buffer start (the rescan walking past bar 0 /
//!                    past a short stitched window) hits the front guard;
//!   · tail-flush   → data ends at the readable region, so any read past the
//!                    chunk's last bar hits the rear guard.
//! Both are run for every (plan, period, split), plus the per-bar `tick` path,
//! which is the most aggressive anchor consumer (every bar re-stitches).
//!
//! The last `PROBE …` line before a fault names the configuration; the fault
//! address relative to `BASE …` names the bar index that was read.
//!
//! Dev-only diagnostic. `cargo run --features should_log_to_db -p tulip_test
//! --example minmax_guard_probe` (or without, for the synthetic fallback).

use std::ffi::c_void;
use std::hint::black_box;
use std::io::Write;
use std::{ptr, slice};

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicators::aroon::Aroon;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::min::Min;
use tulip_rs::indicators::willr::Willr;

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
}

fn page_size() -> usize {
    4096 // x86-64 Linux
}

/// One guarded series: `[guard][readable … data …][guard]`.
struct Guarded {
    base: *mut u8,
    total: usize,
    data: *const f64,
    n: usize,
}

impl Guarded {
    fn new(vals: &[f64], tail_flush: bool) -> Self {
        let n = vals.len();
        let bytes = n * 8;
        let pg = page_size();
        let data_pages = bytes.div_ceil(pg);
        let total = (data_pages + 2) * pg;
        unsafe {
            let p = mmap(
                ptr::null_mut(),
                total,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(
                p as isize != -1 && !p.is_null(),
                "mmap failed ({total} bytes)"
            );
            let p = p as *mut u8;
            assert_eq!(mprotect(p as *mut c_void, pg, PROT_NONE), 0, "front guard");
            assert_eq!(
                mprotect(p.add(total - pg) as *mut c_void, pg, PROT_NONE),
                0,
                "rear guard"
            );
            let lo = p.add(pg) as usize;
            let hi = p.add(total - pg) as usize; // exclusive
            let data = if tail_flush { hi - bytes } else { lo } as *mut f64;
            ptr::copy_nonoverlapping(vals.as_ptr(), data, n);
            Guarded {
                base: p,
                total,
                data: data as *const f64,
                n,
            }
        }
    }

    fn slice(&self) -> &[f64] {
        unsafe { slice::from_raw_parts(self.data, self.n) }
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        // Leak-on-drop is fine for a probe, but unmap so repeated runs stay tidy.
        extern "C" {
            fn munmap(addr: *mut c_void, len: usize) -> i32;
        }
        unsafe {
            let _ = munmap(self.base as *mut c_void, self.total);
        }
    }
}

fn mark(plan: &str, period: usize, split: &str, gs: &[&Guarded]) {
    let bounds = gs
        .iter()
        .map(|g| format!("{:p}..{:p}", g.data, unsafe { g.data.add(g.n) }))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("PROBE plan={plan} p={period} split={split} BASE {bounds}");
    let _ = std::io::stderr().flush();
}

fn series(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
        .collect()
}

fn main() {
    // Two shapes: one where the rolling MIN is always stale (rising → rescan
    // nearly every bar) and one wiggly (rescans occasionally, ties reset it).
    let runs: Vec<(&str, Vec<f64>, Vec<f64>)> = vec![
        (
            "rising",
            (0..400).map(|i| i as f64).collect(),
            (0..400).map(|i| i as f64 - 1.0).collect(),
        ),
        ("wiggle", series(400), {
            let h = series(400);
            h.iter().map(|v| v * 0.97 - 2.0).collect()
        }),
    ];

    for tail_flush in [false, true] {
        let mode = if tail_flush {
            "tail-flush"
        } else {
            "head-flush"
        };
        for (shape, hi, lo) in &runs {
            let n = hi.len();
            let close: Vec<f64> = (0..n).map(|i| lo[i] + (hi[i] - lo[i]) * 0.5).collect();
            let gh = Guarded::new(hi, tail_flush);
            let gl = Guarded::new(lo, tail_flush);
            let gc = Guarded::new(&close, tail_flush);
            let h = gh.slice();
            let l = gl.slice();
            let c = gc.slice();

            // (label, builder, number of streams the plan reads)
            let plans: Vec<(&str, usize, Vec<usize>)> = vec![
                ("max", 1, vec![2, 5, 14, 50]),
                ("min", 1, vec![2, 5, 14, 50]),
                ("donchian", 2, vec![7, 14, 25]),
                ("willr", 3, vec![7, 14, 25]),
                ("aroon", 2, vec![7, 14, 25]),
            ];
            for (label, nfields, periods) in plans {
                for period in periods {
                    let opt = [period as f64];
                    let req = match label {
                        "max" => Request::new().add(Max::recipe(&opt), None),
                        "min" => Request::new().add(Min::recipe(&opt), None),
                        "donchian" => Request::new().add(DonchianChannel::recipe(&opt), None),
                        "willr" => Request::new().add_all(Willr::recipe(&opt)),
                        _ => Request::new().add(Aroon::recipe(&opt), None),
                    };
                    let plan = req.build();
                    let streams: Vec<&[f64]> = match nfields {
                        1 => vec![h],
                        2 => vec![h, l],
                        _ => vec![h, l, c],
                    };
                    let gs: Vec<&Guarded> = match nfields {
                        1 => vec![&gh],
                        2 => vec![&gh, &gl],
                        _ => vec![&gh, &gl, &gc],
                    };
                    let s: Vec<&[f64]> = streams.clone();
                    let refs: &[&[f64]] = &s;

                    mark(label, period, &format!("{mode}/{shape}/cold"), &gs);
                    let (_r, st0) = plan.run(refs);
                    black_box(st0.next_bar());

                    for split in [period - 1, period, period + 1, n / 3, n - 1] {
                        let cut: Vec<&[f64]> = refs.iter().map(|x| &x[..split]).collect();
                        let tail: Vec<&[f64]> = refs.iter().map(|x| &x[split..]).collect();
                        mark(
                            label,
                            period,
                            &format!("{mode}/{shape}/resume@{split}"),
                            &gs,
                        );
                        let (_r1, qs) = plan.run(&cut);
                        let (_r2, st) = plan.resume(qs, &tail);
                        black_box(st.next_bar());
                    }

                    // tick: one bar at a time, so every bar re-stitches.
                    mark(label, period, &format!("{mode}/{shape}/tick"), &gs);
                    let mut st = plan.new_state();
                    let mut out = [0.0f64; 8];
                    let nslots = plan.n_slots().min(8);
                    for i in 0..n {
                        let bar: Vec<&[f64]> = refs.iter().map(|x| &x[i..i + 1]).collect();
                        plan.tick(&mut st, &bar, &mut out[..nslots]);
                        black_box(out[0]);
                    }
                }
            }
        }
    }
    println!("ALL SEQUENCES SURVIVED THE GUARD PAGES");
}
