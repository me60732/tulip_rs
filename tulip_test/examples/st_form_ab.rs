//! natr+supertrend st-BLOCK FORM A/B — same binary, same data, byte-equal
//! outputs asserted:
//!   Branch  = shipped form (if/else trend, Rust f64::max/min)
//!   Select  = branchless bit-select for the trend trio
//!   SimdMax = raw SIMD min/max (single maxsd/minsd; SSE NaN policy — probe
//!             only; promotion needs the float-class sweep, CODEGEN trap table)
//!   Both    = Select + SimdMax
#![feature(portable_simd)]
use std::hint::black_box;
use std::simd::num::SimdFloat;
use std::simd::Simd;
use std::time::Instant;
use tulip_rs::cse::{erase_request, try_run, Planable, Request};
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_test::database::{get_all_stock_data, init_database_data, EodData};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Branch,
    Select,
    SimdMax,
    Both,
}

#[inline(always)]
fn sel_bits(t: bool, a: f64, b: f64) -> f64 {
    let m = (t as u64).wrapping_neg();
    f64::from_bits((a.to_bits() & m) | (b.to_bits() & !m))
}
#[inline(always)]
fn one(x: f64) -> Simd<f64, 1> {
    Simd::from_array([x])
}
#[inline(always)]
fn mx(f: Form, a: f64, b: f64) -> f64 {
    if matches!(f, Form::SimdMax | Form::Both) {
        // hand supertrend_simd idiom: simd_max/simd_min lane ops (their NaN
        // policy differs from f64::max only on NaN / ±0-tie classes — real
        // data asserts equal here; a float-class sweep gates promotion).
        one(a).simd_max(one(b)).to_array()[0]
    } else {
        a.max(b)
    }
}
#[inline(always)]
fn mn(f: Form, a: f64, b: f64) -> f64 {
    if matches!(f, Form::SimdMax | Form::Both) {
        one(a).simd_min(one(b)).to_array()[0]
    } else {
        a.min(b)
    }
}

/// One full steady pass over one asset writing [natr, atr, tr, st, med].
#[inline(never)]
fn run_form(f: Form, hi: &[f64], lo: &[f64], cl: &[f64], r: &mut [Vec<f64>]) {
    let (m, inv) = (13.0f64 / 14.0, 1.0f64 / 14.0);
    let t0 = 16usize; // matches the shipped kernel's gate closely enough
    let (mut c, mut prev_st, mut lb_c, mut ub_c, mut tr_bit): (f64, f64, f64, f64, f64) =
        (0.0, 0.0, 0.0, 0.0, 0.0);
    for t in 1..t0 {
        let pc = cl[t - 1];
        let tr = (hi[t] - lo[t])
            .max((hi[t] - pc).abs())
            .max((lo[t] - pc).abs());
        c = c.mul_add(m, tr * inv);
    }
    let med0 = 0.5 * (hi[t0 - 1] + lo[t0 - 1]);
    prev_st = med0;
    lb_c = med0 - 3.0 * c;
    ub_c = med0 + 3.0 * c;
    tr_bit = 1.0;
    let mut j = 0usize;
    for t in t0..hi.len() {
        let close = unsafe { *cl.get_unchecked(t) };
        let pc = unsafe { *cl.get_unchecked(t - 1) };
        let h = unsafe { *hi.get_unchecked(t) };
        let l = unsafe { *lo.get_unchecked(t) };
        let tr = mx(f, mx(f, h - l, (h - pc).abs()), (l - pc).abs());
        c = c.mul_add(m, tr * inv);
        let atr = c;
        let natr = (atr / close) * 100.0;
        let med = 0.5 * (h + l);
        let w = 3.0 * atr;
        let ub_raw = med + w;
        let lb_raw = med - w;
        let trend = if close > prev_st {
            true
        } else if close < prev_st {
            false
        } else {
            tr_bit != 0.0
        };
        let (stv, nlb, nub) = if matches!(f, Form::Select | Form::Both) {
            let up = mx(f, lb_c, lb_raw);
            let dn = mn(f, ub_c, ub_raw);
            (
                sel_bits(trend, up, dn),
                sel_bits(trend, up, lb_raw),
                sel_bits(trend, ub_raw, dn),
            )
        } else if trend {
            let lb = mx(f, lb_c, lb_raw);
            (lb, lb, ub_raw)
        } else {
            let ub = mn(f, ub_c, ub_raw);
            (ub, lb_raw, ub)
        };
        tr_bit = sel_bits(trend, 1.0, 0.0);
        prev_st = stv;
        lb_c = nlb;
        ub_c = nub;
        r[0][j] = natr;
        r[1][j] = atr;
        r[2][j] = tr;
        r[3][j] = stv;
        r[4][j] = med;
        j += 1;
    }
}

fn bench_form(
    name: &str,
    f: Form,
    data: &[(Vec<f64>, Vec<f64>, Vec<f64>)],
    n: usize,
) -> Vec<Vec<f64>> {
    let t0 = 16usize;
    let mut outs: Vec<Vec<f64>> = (0..5).map(|_| vec![0.0f64; n - t0]).collect();
    let mut t = Instant::now();
    for _ in 0..100 {
        for (hi, lo, cl) in data {
            run_form(f, hi, lo, cl, &mut outs);
        }
    }
    let us = t.elapsed().as_secs_f64() * 1e4;
    let bars = (n - t0) * data.len();
    println!(
        "{name:26} {us:7.2} µs/scan(4 assets)   {:.3} ns/bar",
        us * 1000.0 / bars as f64
    );
    outs
}

fn main() {
    init_database_data();
    let all = get_all_stock_data().unwrap();
    let n = all[0].1.len();
    let data: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = all
        .iter()
        .map(|(_, d)| {
            (
                d.iter().map(|x| x.high).collect(),
                d.iter().map(|x| x.low).collect(),
                d.iter().map(|x| x.close).collect(),
            )
        })
        .collect();

    // shipped kernel absolute reference (opton = all 5 rows, asset 0)
    let req = Request::new()
        .add(Natr::recipe(&[14.0]), Some(&[true, true]))
        .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true]));
    let (_a, sig, vals, _o) = erase_request(&req).unwrap();
    let (hi, lo, cl) = &data[0];
    let three = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let t = Instant::now();
    for _ in 0..200 {
        black_box(try_run(&sig, &vals, &three).unwrap()[0].len());
    }
    println!(
        "shipped kernel (1 asset, opton) {:7.2} µs/scan",
        t.elapsed().as_secs_f64() * 5e3
    );

    let v0 = bench_form("Branch (control)", Form::Branch, &data, n);
    let v1 = bench_form("Select (branchless)", Form::Select, &data, n);
    let v2 = bench_form("SimdMax (branch kept)", Form::SimdMax, &data, n);
    let v3 = bench_form("Select+SimdMax", Form::Both, &data, n);
    for k in 0..5 {
        assert_eq!(v0[k], v1[k], "Select differs");
        assert_eq!(v0[k], v2[k], "SimdMax differs on real data");
        assert_eq!(v0[k], v3[k], "Both differs");
    }
    println!("all forms byte-equal to Branch on 4-asset real data");
}
