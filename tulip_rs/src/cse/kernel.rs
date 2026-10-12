//! The specialized fused interpreter: `run::<SIG>` walks a [`Signature`]'s
//! SSA arena per bar with a scratch array the size of the pattern — with the
//! signature a const generic, LLVM unrolls the loop, folds every `match` to
//! one arm per slot, SROAs the scratch into registers, and hoists the
//! [`Values`] loads (loop-invariant) — the same straight-line loop the JIT
//! emitted at runtime, emitted by the optimizer at compile time instead.
//!
//! M0 scope: single cold run of a whole series (`run`), scalar (one asset);
//! typed-state resume and `get_states` are M1, `L > 1` lanes M2 — the body
//! below is what both wrap.
//!
//! Bit-exact forms (mirroring the hand modules, the spec per doctrine):
//! * `Ema`: `prev.mul_add(inv, src * mult)` — `ema::calc` exactly; seed =
//!   source value at its feed bar (`0` for price-sourced: bar 0, mirroring
//!   `init_state`'s recursion from `real[0]`).
//! * `Wilders` mean-mode: seed = `sum(first p)/p` accumulated in bar order,
//!   then `prev.mul_add(mult, src * inv)`; composite keeps the raw sum.
//! * `Max` = IEEE maxNum via `f64::max` (the hand's own NaN rule).

use crate::cse::signature::{
    Signature, Values, K_ABS, K_ADD, K_CONST, K_DIV, K_EMA, K_FMA, K_INPUT, K_LAG, K_MAX, K_MIRROR,
    K_MUL, K_ROLLMAX, K_ROLLMIN, K_SMA, K_SQRT, K_STDDEV, K_SUB, K_SUPERTREND, K_TRAIL, K_WHEREGE,
    K_WHERELT, K_WILDERS, K_WMA, MAX, MAX_SINKS,
};

/// One trail-rescan roll step — the hand's `max`/`min` `State` logic
/// transcribed literally, `find_max_simd::<4>`/`find_min_simd::<4>` called
/// for the rescan so the tie/plateau rules cannot drift by construction:
/// bars `0..lb` fold (`init_state`: seed at 0, then `>=`/`<=` replaces the
/// extremum and zeroes the trail, else trail += 1); from bar `lb` the calc
/// rule (`trail += 1`; `p <= trail` RESCANs the `lb + 1`-bar window; else
/// the entering bar takes the throne on `>=`/`<=`). `MAXD` selects the
/// direction; p = trigger period, lb = look_back (window = lb + 1 bars).
#[inline(always)]
fn roll_step<const MAXD: bool>(
    src: &[f64],
    t: usize,
    p: usize,
    lb: usize,
    val: &mut f64,
    trail: &mut usize,
) -> f64 {
    if t < lb {
        if t == 0 {
            *val = src[0];
            *trail = 0;
        } else {
            let bar = src[t];
            let hit = if MAXD { bar >= *val } else { bar <= *val };
            if hit {
                *val = bar;
                *trail = 0;
            } else {
                *trail += 1;
            }
        }
        0.0
    } else {
        *trail += 1;
        if p <= *trail {
            let window = &src[t - lb..=t];
            let (v, idx) = if MAXD {
                crate::indicators::max::find_max_simd::<4>(window)
            } else {
                crate::indicators::min::find_min_simd::<4>(window)
            };
            *val = v;
            *trail = lb - idx;
        } else {
            let current = src[t];
            let hit = if MAXD {
                current >= *val
            } else {
                current <= *val
            };
            if hit {
                *val = current;
                *trail = 0;
            }
        }
        *val
    }
}

/// Run pattern `SIG` over `streams` (M0 patterns are single-stream). Returns
/// one row per sink, in arena sink order, each `n − lead` long — the hand
/// rows' exact shape and values.
pub fn run<const SIG: Signature>(vals: &Values, streams: &[&[f64]]) -> Vec<Vec<f64>> {
    let n = streams.iter().map(|s| s.len()).min().unwrap_or(0);
    let gate = (vals.max_gate as usize + 1).min(n);
    // Build rows per sink: disabled sinks get zero-length empty Vec
    let mut out: Vec<Vec<f64>> = (0..SIG.n_sinks as usize)
        .map(|s| {
            if vals.sink_want[s] {
                let len = n.saturating_sub(vals.sink_lead[s] as usize);
                crate::uninit_vec!(f64, len)
            } else {
                Vec::new() // disabled sink: zero-length empty row
            }
        })
        .collect();
    if n == 0 {
        return out;
    }
    // Established idiom (by_asset drivers): the driver writes through raw
    // row pointers while the Vecs below own them; every index [0, len) is
    // written exactly once (warm covers t < lead rows' heads, steady the
    // tail — t-lead is surjective over each row by construction).
    let mut ptrs: [*mut f64; MAX_SINKS] = [std::ptr::null_mut(); MAX_SINKS];
    for (s, row) in out.iter_mut().enumerate() {
        if vals.sink_want[s] && !row.is_empty() {
            ptrs[s] = row.as_mut_ptr();
        }
        // disabled sinks: ptr stays null_mut; exec guards skip deref
    }
    let mut st = [0.0f64; MAX];
    let mut stb = [0.0f64; MAX]; // SuperTrend prev_st
    let mut stc_ub = [0.0f64; MAX]; // SuperTrend prev_ub
    let mut stlb = [0.0f64; MAX]; // SuperTrend prev_lb
    let mut sumsq = [0.0f64; MAX]; // Stddev sum-of-squares carry
    let mut wsum = [0.0f64; MAX]; // Wma weighted-sum carry
    let mut trail = [0usize; MAX]; // roll trail carry
    let mut v = [0.0f64; MAX];
    // Warm: [0, gate) — seeds and per-sink lead gates live here (all chain
    // feeds are <= max_gate < gate, so the steady loop below needs neither).
    exec::<SIG, true>(
        vals,
        streams,
        0,
        gate,
        &mut st,
        &mut stb,
        &mut stc_ub,
        &mut stlb,
        &mut sumsq,
        &mut wsum,
        &mut trail,
        &mut v,
        &mut ptrs,
        &vals.sink_lead,
    );
    // Steady: [gate, n) — every sink publishes, every chain steps.
    exec::<SIG, false>(
        vals,
        streams,
        gate,
        n,
        &mut st,
        &mut stb,
        &mut stc_ub,
        &mut stlb,
        &mut sumsq,
        &mut wsum,
        &mut trail,
        &mut v,
        &mut ptrs,
        &vals.sink_lead,
    );
    out
}

#[inline(always)]
pub fn exec<const SIG: Signature, const WARM: bool>(
    vals: &Values,
    streams: &[&[f64]],
    t0: usize,
    t1: usize,
    st: &mut [f64; MAX],
    stb: &mut [f64; MAX],     // SuperTrend prev_st
    stc_ub: &mut [f64; MAX],  // SuperTrend prev_ub
    stlb: &mut [f64; MAX],    // SuperTrend prev_lb
    sumsq: &mut [f64; MAX],   // Stddev sum-of-squares carry
    wsum: &mut [f64; MAX],    // Wma weighted-sum carry
    trail: &mut [usize; MAX], // RollMax/RollMin trail carry (Trail reads it)
    v: &mut [f64; MAX],
    out: &mut [*mut f64; MAX_SINKS],
    leads: &[u32; MAX_SINKS],
) {
    // Iteration over the CONST capacity with a const `break` per slot: every
    // iteration's `k` is a literal for the optimizer, so `SIG.kinds[k]` folds
    // to a constant and the `match` collapses to its single arm WITHOUT
    // relying on unroll heuristics (the M0 lesson: LLVM's heuristics refused
    // to unroll this body over the runtime `SIG.len`, leaving the dispatch
    // loop intact — 1.16x SLOWER than hand). `break` bounds it to the real
    // arena; the tail is dead code.
    for t in t0..t1 {
        for k in 0..MAX {
            if k == SIG.len as usize {
                break;
            }
            v[k] = match SIG.kinds[k] {
                K_INPUT => streams[SIG.a[k] as usize][t],
                K_CONST => vals.consts[k],
                K_EMA => {
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else if t == feed {
                        st[k] = s;
                        s
                    } else {
                        st[k] = st[k].mul_add(vals.invms[k], s * vals.mults[k]);
                        st[k]
                    }
                }
                K_WILDERS => {
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    let p = vals.periods[k] as usize;
                    if t < feed {
                        0.0
                    } else if t - feed < p - 1 {
                        st[k] += s;
                        0.0
                    } else if t - feed == p - 1 && SIG.b[k] == 0 {
                        st[k] = (st[k] + s) / p as f64;
                        st[k]
                    } else if t - feed == p - 1 {
                        st[k] += s;
                        st[k]
                    } else {
                        st[k] = st[k].mul_add(vals.mults[k], s * vals.invms[k]);
                        st[k]
                    }
                }
                K_SUB => v[SIG.a[k] as usize] - v[SIG.b[k] as usize],
                K_ADD => v[SIG.a[k] as usize] + v[SIG.b[k] as usize],
                K_MUL => v[SIG.a[k] as usize] * v[SIG.b[k] as usize],
                K_DIV => v[SIG.a[k] as usize] / v[SIG.b[k] as usize],
                K_MAX => v[SIG.a[k] as usize].max(v[SIG.b[k] as usize]),
                K_ABS => v[SIG.a[k] as usize].abs(),
                K_MIRROR => v[SIG.a[k] as usize],
                K_SQRT => v[SIG.a[k] as usize].sqrt(),
                K_SMA => {
                    // hand `cycle_sma`: `init_state` folds bars `0..p`
                    // (`sum += real[i]` in order), then from bar `p` every
                    // step is `sum += real[i] − real[i−p]` and the value is
                    // `sum · (1/p)`. The window's leaving bar reads the
                    // stream directly (price source only — erase guarantees
                    // it), so no anchor is needed for a whole-series run.
                    let p = vals.periods[k] as usize;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        st[k] += src[t];
                        0.0
                    } else {
                        st[k] += src[t] - src[t - p];
                        st[k] * vals.mults[k]
                    }
                }
                K_STDDEV => {
                    // hand `stddev::State::calc` op for op: the sma step
                    // first (its own sum carry), then the sum-of-squares
                    // step, then the two-moment variance in the hand's
                    // association (`mul_add` for both differences), and the
                    // flat-bar floor `sqrt(var).max(ε)` — a negative var
                    // from rounding or a flat window is legal data, so the
                    // ε stays (per the `safe_inputs` retirement note).
                    let p = vals.periods[k] as usize;
                    let m = vals.mults[k];
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        let s = src[t];
                        st[k] += s;
                        sumsq[k] = s.mul_add(s, sumsq[k]);
                        0.0
                    } else {
                        let s = src[t];
                        let prev = src[t - p];
                        st[k] += s - prev;
                        let mean = st[k] * m;
                        sumsq[k] += s.mul_add(s, -(prev * prev));
                        let var = sumsq[k].mul_add(m, -(mean * mean));
                        var.sqrt().max(f64::EPSILON)
                    }
                }
                K_WMA => {
                    // hand `wma::State::calc` in its EXACT order: the
                    // weighted line first drops the PRE-update window sum,
                    // then the sum steps, then the entering bar lands at
                    // weight `n`, then the one multiply by the weight
                    // reciprocal `2/(n(n+1))`. `init_state` folds bars
                    // `0..p` with weights `1..=p` (entering index + 1).
                    let p = vals.periods[k] as usize;
                    let n = vals.periods[k] as f64;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        let value = src[t];
                        st[k] += value;
                        wsum[k] += value * (t + 1) as f64;
                        0.0
                    } else {
                        let value = src[t];
                        let prev = src[t - p];
                        wsum[k] -= st[k];
                        st[k] += value - prev;
                        wsum[k] += value * n;
                        wsum[k] * vals.mults[k]
                    }
                }
                K_FMA => v[SIG.a[k] as usize].mul_add(v[SIG.b[k] as usize], v[SIG.c[k] as usize]),
                K_ROLLMAX => {
                    let p = vals.periods[k] as usize;
                    let lb = vals.roll_lb[k] as usize;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    roll_step::<true>(src, t, p, lb, &mut st[k], &mut trail[k])
                }
                K_ROLLMIN => {
                    let p = vals.periods[k] as usize;
                    let lb = vals.roll_lb[k] as usize;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    roll_step::<false>(src, t, p, lb, &mut st[k], &mut trail[k])
                }
                K_TRAIL => trail[SIG.a[k] as usize] as f64,
                K_WHEREGE => {
                    if v[SIG.a[k] as usize] >= v[SIG.b[k] as usize] {
                        v[SIG.c[k] as usize]
                    } else {
                        0.0
                    }
                }
                K_WHERELT => {
                    // the hand's denominator-guard form: `<` takes the PINNED
                    // zero, `>=` (and NaN — false both ways) takes the value.
                    if v[SIG.a[k] as usize] < v[SIG.b[k] as usize] {
                        0.0
                    } else {
                        v[SIG.c[k] as usize]
                    }
                }
                K_SUPERTREND => {
                    // SuperTrend per bar — ONE uniform formula transcribing the hand
                    // `calc_st` (indicators/supertrend.rs): prev_st=stb[k],
                    // prev_ub=stc_ub[k], prev_lb=stlb[k], and the carried trend BOOL
                    // in the node's own st[k] slot as 1.0/0.0 (an st node carries no
                    // chain value, so that slot is otherwise unused). The arrays are
                    // zero-initialized, which IS the hand's init_state
                    // (prev_st/prev_ub/prev_lb = 0.0, trend = false) — the first bar
                    // past the seam therefore needs no special seed: the clamp with
                    // the zero bands falls out of the same formula.
                    let lead = vals.feeds[k] as usize;
                    if t < lead {
                        0.0 // idle before the atr seam; carries untouched
                    } else {
                        let atr_v = v[SIG.a[k] as usize];
                        let med_v = v[SIG.b[k] as usize];
                        let close_v = v[SIG.c[k] as usize];
                        let w = vals.st_step[k] * atr_v;
                        let mut ub = med_v + w;
                        let mut lb = med_v - w;
                        let crosses_up = close_v > stb[k];
                        let crosses_down = close_v < stb[k];
                        let trend = crosses_up || (st[k] != 0.0 && !crosses_down);
                        let st_val = if trend {
                            lb = stlb[k].max(lb);
                            lb
                        } else {
                            ub = stc_ub[k].min(ub);
                            ub
                        };
                        st[k] = if trend { 1.0 } else { 0.0 };
                        stb[k] = st_val;
                        stlb[k] = lb;
                        stc_ub[k] = ub;
                        st_val
                    }
                }
                K_LAG => {
                    // price stream (erase guarantees); hand clamp: bars < n
                    // read sample 0 (the E6 bar-0 tr trick depends on it).
                    let nn = vals.lag_n[k] as usize;
                    let src = SIG.a[SIG.a[k] as usize] as usize;
                    streams[src][if t >= nn { t - nn } else { 0 }]
                }
                _ => {
                    debug_assert!(false, "kernel: kind {} outside M0 vocabulary", SIG.kinds[k]);
                    0.0
                }
            };
        }
        for s in 0..MAX_SINKS {
            if s == SIG.n_sinks as usize {
                break;
            }
            let lead = leads[s] as usize;
            if WARM && t < lead {
                continue;
            }
            // Guarded store: only write if sink is enabled (runtime want flag)
            unsafe {
                if vals.sink_want[s] {
                    *out[s].add(t - lead) = v[SIG.sink_src[s] as usize];
                }
            }
        }
    }
}
