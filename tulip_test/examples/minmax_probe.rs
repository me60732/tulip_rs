//! E9 M0 diagnostic — WHERE does the roll's time go?
//!
//! Two candidate costs, and they need different fixes:
//!   · the STEADY body (1 load + compare + 2 selects + 1 brif + store) — if
//!     this dominates, the problem is codegen/scheduling;
//!   · the RESCAN, which is data-driven: on a series whose window extremum sits
//!     at the OLDEST bar, `trail` reaches `period` and then stays there, so the
//!     hand's algorithm rescans EVERY bar. The hand rescans with a 4-wide AVX
//!     reduction; cranelift's ceiling is 2-wide, so if this dominates, the fix
//!     is vectorizing the rescan, not touching the steady path.
//!
//! Prints the rescan-bar count (recomputing the hand's exact trail predicate in
//! Rust) next to the JIT's ns/bar for several periods, so the two hypotheses
//! separate. Dev-only; prints, no asserts.

use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicator_types::Indicator;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::min::Min;
use tulip_test::database::{get_all_stock_data, init_database_data};

/// Replicate the hand's per-bar predicate and count rescan bars.
/// `value`/`trail` start exactly as `State::init_state` leaves them (bar 0
/// seeds, bars 1..=lead-1 fold), and the printed row starts at bar `lead`.
fn rescan_count(real: &[f64], period: usize, want_max: bool) -> (usize, usize) {
    let look_back = period - 1;
    let lead = look_back; // first printed bar for standalone max/min
    let mut value = real[0];
    let mut trail = 0usize;
    for i in 1..lead {
        let hit = if want_max {
            real[i] >= value
        } else {
            real[i] <= value
        };
        if hit {
            value = real[i];
            trail = 0;
        } else {
            trail += 1;
        }
    }
    let mut rescans = 0;
    for i in lead..real.len() {
        trail += 1;
        if period <= trail {
            rescans += 1;
            // the hand's window rescan: newest occurrence of the extremum
            let win = &real[(i - look_back)..=i];
            let mut idx = 0;
            for (k, v) in win.iter().enumerate().skip(1) {
                let take = if want_max {
                    *v >= win[idx]
                } else {
                    *v <= win[idx]
                };
                if take {
                    idx = k;
                }
            }
            value = win[idx];
            trail = look_back - idx;
        } else {
            let hit = if want_max {
                real[i] >= value
            } else {
                real[i] <= value
            };
            if hit {
                value = real[i];
                trail = 0;
            }
        }
    }
    (rescans, real.len() - lead)
}

fn main() {
    init_database_data();
    let stocks = get_all_stock_data().expect("no EOD rows loaded");
    let rows = &stocks[0].1;
    let n = rows.len();
    let hi: Vec<f64> = rows.iter().map(|r| r.high).collect();
    let lo: Vec<f64> = rows.iter().map(|r| r.low).collect();
    println!(
        "{}: {n} bars  (high range {}..{}, low range {}..{})",
        stocks[0].0,
        hi.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        hi.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
        lo.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        lo.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
    );

    for p in [3usize, 5, 8, 20, 50, 100] {
        let hinp: [&[f64]; 1] = [hi.as_slice()];
        let lot: [&[f64]; 1] = [lo.as_slice()];
        let (rx, bars) = rescan_count(&hi, p, true);
        let (rn, _) = rescan_count(&lo, p, false);
        let mx = Request::new().add(Max::recipe(&[p as f64]), None).build();
        let mn = Request::new().add(Min::recipe(&[p as f64]), None).build();
        let hand_mx = Max::indicator(&hinp, &[p as f64], None).unwrap().0[0].len();
        let _ = hand_mx;

        // time each side over a fixed iteration count
        let iters = 2000usize;
        let t_jit_mx = timed(iters, || {
            let (r, q) = mx.run(&hinp);
            std::hint::black_box((r.outputs[0][0].len(), q.next_bar()))
        });
        let t_jit_mn = timed(iters, || {
            let (r, q) = mn.run(&lot);
            std::hint::black_box((r.outputs[0][0].len(), q.next_bar()))
        });
        let t_hand_mx = timed(iters, || {
            let v = Max::indicator(&hinp, &[p as f64], None).unwrap();
            std::hint::black_box(v.0[0].len())
        });
        let t_hand_mn = timed(iters, || {
            let v = Min::indicator(&lot, &[p as f64], None).unwrap();
            std::hint::black_box(v.0[0].len())
        });
        let ns = |t: f64| t * 1e9 / iters as f64 / n as f64;
        println!(
            "p={p:>3} rescan bars: max {rx}/{bars} ({:.0}%) min {rn}/{bars} ({:.0}%) | \
             ns/bar  jit_max {:.2} hand_max {:.2} ({:.1}x)  jit_min {:.2} hand_min {:.2} ({:.1}x) | \
             ops {:?} clif {}",
            100.0 * rx as f64 / bars as f64,
            100.0 * rn as f64 / bars as f64,
            ns(t_jit_mx),
            ns(t_hand_mx),
            ns(t_jit_mx) / ns(t_hand_mx),
            ns(t_jit_mn),
            ns(t_hand_mn),
            ns(t_jit_mn) / ns(t_hand_mn),
            mx.info(),
            mx.clif().len(),
        );
    }

    // ── PAIRED-SCAN ceiling. A `RollMinMax` node (ONE walk producing both
    // extremums, fired by EITHER trail) saves loads only on bars where BOTH
    // sides rescan. If the two trails expire on different bars, the union fires
    // almost as often as the two walks do separately and there is little to win.
    // Only the SAME-series pair is a candidate at all — aroon/willr/donchian/
    // stoch take max of high and min of low, i.e. different address ranges.
    println!("\nsame-series max+min: rescan OVERLAP (the ceiling of one shared walk)");
    let hinp: [&[f64]; 1] = [hi.as_slice()];
    for p in [14usize, 20, 52] {
        let lead = p - 1;
        let (mut both, mut only_x, mut only_n, mut bars) = (0usize, 0usize, 0usize, 0usize);
        let (mut vx, mut tx) = (hi[0], 0usize);
        let (mut vn, mut tn) = (hi[0], 0usize);
        for i in 1..lead {
            if hi[i] >= vx {
                vx = hi[i];
                tx = 0;
            } else {
                tx += 1;
            }
            if hi[i] <= vn {
                vn = hi[i];
                tn = 0;
            } else {
                tn += 1;
            }
        }
        for i in lead..n {
            bars += 1;
            tx += 1;
            tn += 1;
            let fx = p <= tx;
            let fmin = p <= tn;
            if fx && fmin {
                both += 1;
            } else if fx {
                only_x += 1;
            } else if fmin {
                only_n += 1;
            }
            macro_rules! roll {
                ($v:ident, $t:ident, $fire:expr, $ge:expr) => {
                    if $fire {
                        let win = &hi[(i - lead)..=i];
                        let mut idx = 0usize;
                        for (k, x) in win.iter().enumerate().skip(1) {
                            let take = if $ge { *x >= win[idx] } else { *x <= win[idx] };
                            if take {
                                idx = k;
                            }
                        }
                        $v = win[idx];
                        $t = lead - idx;
                    } else {
                        let hit = if $ge { hi[i] >= $v } else { hi[i] <= $v };
                        if hit {
                            $v = hi[i];
                            $t = 0;
                        }
                    }
                };
            }
            roll!(vx, tx, fx, true);
            roll!(vn, tn, fmin, false);
        }
        // walks executed: separate = one per side per fired bar; paired = one per
        // bar in the union. Both walk `p` loads, so this ratio is the load saving.
        let sep = (2 * both + only_x + only_n) as f64;
        let paired = (both + only_x + only_n) as f64;
        println!(
            "  p={p:>2} bars {bars}: both {both} ({:.0}%) max-only {only_x} min-only {only_n} \
             | walks {sep:.0} separate → {paired:.0} paired = {:.0}% of rescan loads removed \
             (union fires {:.2}/bar)",
            100.0 * both as f64 / bars as f64,
            100.0 * (sep - paired) / sep,
            (both + only_x + only_n) as f64 / bars as f64,
        );

        // The shape a paired node would actually replace, timed.
        let opt = [p as f64];
        let solo = Request::new().add(Max::recipe(&opt), None).build();
        let pair_same = Request::new()
            .add(Max::recipe(&opt), None)
            .add(Min::recipe(&opt), None)
            .build();
        assert_eq!(
            pair_same.n_streams(),
            1,
            "both requests read field 0 — the SAME window for both"
        );
        let t_max = timed(1500, || {
            let (r, q) = solo.run(&hinp);
            std::hint::black_box((r.outputs[0][0].len(), q.next_bar()))
        });
        let t_pair = timed(1500, || {
            let (r, q) = pair_same.run(&hinp);
            std::hint::black_box((r.outputs[1][0].len(), q.next_bar()))
        });
        let ns = |t: f64| t * 1e9 / 1500.0 / n as f64;
        println!(
            "        max alone {:.2} ns/bar | max+min same series {:.2} (marginal +{:.2}) | ops {:?}",
            ns(t_max),
            ns(t_pair),
            ns(t_pair) - ns(t_max),
            pair_same.info(),
        );
    }
}

fn timed<T>(iters: usize, mut f: impl FnMut() -> T) -> f64 {
    // warm up (page faults + plan cache) then time
    for _ in 0..50 {
        std::hint::black_box(f());
    }
    let t0 = std::time::Instant::now();
    for _ in 0..iters {
        std::hint::black_box(f());
    }
    t0.elapsed().as_secs_f64()
}
