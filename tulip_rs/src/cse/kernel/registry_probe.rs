//! Probe-only mini kernel registry for focused code-shape inspection.
//!
//! Enabled only with `--features cse_kernel_probe`.
//! This module is intentionally not wired into production routing.

tulip_rs_macros::kernel! {
    // Singleton outliers under investigation.
    probe_ao_s_m: Ao(s, m);
    probe_di_n: Di(p);
    probe_dm_di_dx_adx_shared_p: Dm(p), Di(p), Dx(p), Adx(p);
    probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base:
        Dm(p), Di(p), Dx(p), Adx(p), Atr(p), Natr(p), Supertrend(p, 3.0);
    probe_stoch_kd: Stoch(k, s, d);
    probe_cmo_n: Cmo(p);
    probe_ultosc_s_m_l: Ultosc(s, m, l);
}

/// Probe-only specialized AO kernel candidate.
///
/// Intentionally feature-gated (`cse_kernel_probe`) and not used by production
/// routing. Mirrors AO's hand recurrence shape directly (two rolling sums over
/// medprice) while preserving CSE row ordering and optional sink semantics.
pub fn probe_ao_s_m_fast(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    let n = streams.iter().map(|s| s.len()).min().unwrap_or(0);
    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
        ];
    }

    let leads = [
        vals.sink_lead[0] as usize,
        vals.sink_lead[1] as usize,
        vals.sink_lead[2] as usize,
        vals.sink_lead[3] as usize,
    ];
    let wants = [
        vals.sink_want[0],
        vals.sink_want[1],
        vals.sink_want[2],
        vals.sink_want[3],
    ];

    let rsz0 = n.saturating_sub(leads[0]);
    let rsz1 = n.saturating_sub(leads[1]);
    let rsz2 = n.saturating_sub(leads[2]);
    let rsz3 = n.saturating_sub(leads[3]);
    let off0 = 0usize;
    let off1 = off0 + rsz0;
    let off2 = off1 + rsz1;
    let off3 = off2 + rsz2;
    let total = rsz0 + rsz1 + rsz2 + rsz3;

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
    let base: *mut f64 = if total == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    let row0 = unsafe { base.add(off0) };
    let row1 = unsafe { base.add(off1) };
    let row2 = unsafe { base.add(off2) };
    let row3 = unsafe { base.add(off3) };

    let short_idx = 5usize;
    let long_idx = 6usize;
    let half_idx = 3usize;

    let short_p = vals.periods[short_idx] as usize;
    let long_p = vals.periods[long_idx] as usize;
    let short_mult = vals.mults[short_idx];
    let long_mult = vals.mults[long_idx];
    let half = vals.consts[half_idx];

    let high = streams[0];
    let low = streams[1];

    let mut med_hist = vec![0.0_f64; n];
    let mut short_sum = 0.0_f64;
    let mut long_sum = 0.0_f64;

    for t in 0..n {
        let med = half * (high[t] + low[t]);
        med_hist[t] = med;

        if t < short_p {
            short_sum += med;
        } else {
            short_sum += med - med_hist[t - short_p];
        }

        if t < long_p {
            long_sum += med;
        } else {
            long_sum += med - med_hist[t - long_p];
        }

        let sma_short = short_sum * short_mult;
        let sma_long = long_sum * long_mult;
        let ao = sma_short - sma_long;

        if wants[0] && t >= leads[0] {
            unsafe { *row0.add(t - leads[0]) = ao };
        }
        if wants[1] && t >= leads[1] {
            unsafe { *row1.add(t - leads[1]) = sma_short };
        }
        if wants[2] && t >= leads[2] {
            unsafe { *row2.add(t - leads[2]) = sma_long };
        }
        if wants[3] && t >= leads[3] {
            unsafe { *row3.add(t - leads[3]) = med };
        }
    }

    vec![
        if wants[0] {
            crate::cse::query::RowRef::window(&arena, off0, rsz0)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
        if wants[1] {
            crate::cse::query::RowRef::window(&arena, off1, rsz1)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
        if wants[2] {
            crate::cse::query::RowRef::window(&arena, off2, rsz2)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
        if wants[3] {
            crate::cse::query::RowRef::window(&arena, off3, rsz3)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
    ]
}

/// Probe-only specialized DM/DI/DX/ADX shared-p kernel candidate.
///
/// Uses hand-like recurrences for the directional movement hub (DM + DI + DX + ADX)
/// while preserving CSE sink ordering and `RowRef` semantics. Feature-gated
/// (`cse_kernel_probe`) and intentionally not used by production routing.
pub fn probe_dm_di_dx_adx_shared_p_fast(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct LockedMode {
        use_fast: bool,
    }

    #[inline(always)]
    fn calc_dp_dm(prev_high: &mut f64, prev_low: &mut f64, high: f64, low: f64) -> (f64, f64) {
        let mut dp = high - *prev_high;
        let mut dm = *prev_low - low;
        *prev_high = high;
        *prev_low = low;

        if dp < 0.0 {
            dp = 0.0;
        } else if dp > dm {
            dm = 0.0;
        }
        if dm < 0.0 {
            dm = 0.0;
        } else if dm > dp {
            dp = 0.0;
        }
        (dp, dm)
    }

    #[inline(always)]
    fn calc_dx(dmup: f64, dmdown: f64) -> f64 {
        let diff = (dmup - dmdown).abs();
        let sum = dmup + dmdown;
        (diff * 100.0 / sum).max(0.0)
    }

    static LOCKED_MODE: std::sync::OnceLock<LockedMode> = std::sync::OnceLock::new();
    const TOLERANCE: f64 = 1e-10;

    if streams.len() < 3 {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    let (high, low, close) = (streams[0], streams[1], streams[2]);
    let n = high.len().min(low.len()).min(close.len());

    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
        ];
    }

    if high.len() != n || low.len() != n || close.len() != n {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    if vals.sink_want.len() < 6 || vals.sink_lead.len() < 6 {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }
    if vals.sink_want.iter().skip(6).any(|w| *w) {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    let want_primary_all = vals.sink_want[0]
        && vals.sink_want[1]
        && vals.sink_want[2]
        && vals.sink_want[3]
        && vals.sink_want[4]
        && vals.sink_want[5];
    if !want_primary_all {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    let period = vals.sink_lead[0] as usize;
    if period == 0 {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }
    let adx_lead = period.saturating_mul(2).saturating_sub(1);
    if vals.sink_lead[1] as usize != period
        || vals.sink_lead[2] as usize != period
        || vals.sink_lead[3] as usize != period
        || vals.sink_lead[4] as usize != period
        || vals.sink_lead[5] as usize != adx_lead
    {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    let mode = *LOCKED_MODE.get_or_init(|| {
        let baseline = probe_dm_di_dx_adx_shared_p(vals, streams);
        if baseline.len() < 6 {
            return LockedMode { use_fast: false };
        }

        let szp = n.saturating_sub(period);
        let sza = n.saturating_sub(adx_lead);
        if baseline[0].len() != szp
            || baseline[1].len() != szp
            || baseline[2].len() != szp
            || baseline[3].len() != szp
            || baseline[4].len() != szp
            || baseline[5].len() != sza
        {
            return LockedMode { use_fast: false };
        }

        let mult = (period - 1) as f64 / period as f64;
        let inv_p = 1.0 / period as f64;
        let di_scale = 100.0 * inv_p;

        let mut dmup = 0.0_f64;
        let mut dmdown = 0.0_f64;
        let mut prev_high = high[0];
        let mut prev_low = low[0];
        for i in 1..period {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup += dp;
            dmdown += dm;
        }

        let mut prev_close = close[0];
        let mut atr_sum = high[0] - low[0];
        for i in 1..period {
            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            atr_sum += hl.max(hpc).max(lpc);
            prev_close = close[i];
        }
        let mut atr_mean = atr_sum * inv_p;

        let mut adx_acc = calc_dx(dmup, dmdown);
        let mut adx_state = f64::NAN;
        let mut err = 0.0_f64;

        for i in period..n {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup = dmup.mul_add(mult, dp);
            dmdown = dmdown.mul_add(mult, dm);

            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            let tr = hl.max(hpc).max(lpc);
            prev_close = close[i];
            atr_mean = atr_mean.mul_add(mult, tr * inv_p);

            let atr_inv = di_scale / atr_mean;
            let pdi = (dmup * atr_inv).max(0.0);
            let mdi = (dmdown * atr_inv).max(0.0);
            let dx = calc_dx(dmup, dmdown);

            let kp = i - period;
            let cands = [dmup, dmdown, pdi, mdi, dx];
            for row in 0..5usize {
                let a = cands[row];
                let b = baseline[row][kp];
                if a.is_finite() && b.is_finite() {
                    err = err.max((a - b).abs());
                }
            }

            if i < adx_lead {
                adx_acc += dx;
                if i + 1 == adx_lead {
                    adx_state = adx_acc * inv_p;
                }
            } else {
                if !adx_state.is_finite() {
                    adx_state = adx_acc * inv_p;
                }
                adx_state = adx_state.mul_add(mult, dx * inv_p);
                let ka = i - adx_lead;
                let b = baseline[5][ka];
                if adx_state.is_finite() && b.is_finite() {
                    err = err.max((adx_state - b).abs());
                }
            }
        }

        let locked = LockedMode {
            use_fast: err <= TOLERANCE,
        };

        eprintln!(
            "probe_dm_di_dx_adx_shared_p_fast_mode,use_fast={},err={:.12},tolerance={:.12}",
            locked.use_fast, err, TOLERANCE,
        );

        locked
    });

    if !mode.use_fast {
        return probe_dm_di_dx_adx_shared_p(vals, streams);
    }

    let szp = n.saturating_sub(period);
    let sza = n.saturating_sub(adx_lead);
    let off0 = 0usize;
    let off1 = off0 + szp;
    let off2 = off1 + szp;
    let off3 = off2 + szp;
    let off4 = off3 + szp;
    let off5 = off4 + szp;
    let total = off5 + sza;

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
    let base: *mut f64 = if total == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    let row0 = unsafe { base.add(off0) };
    let row1 = unsafe { base.add(off1) };
    let row2 = unsafe { base.add(off2) };
    let row3 = unsafe { base.add(off3) };
    let row4 = unsafe { base.add(off4) };
    let row5 = unsafe { base.add(off5) };

    let mult = (period - 1) as f64 / period as f64;
    let inv_p = 1.0 / period as f64;
    let di_scale = 100.0 * inv_p;

    let mut dmup = 0.0_f64;
    let mut dmdown = 0.0_f64;
    let mut prev_high = high[0];
    let mut prev_low = low[0];
    for i in 1..period {
        let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
        dmup += dp;
        dmdown += dm;
    }

    let mut prev_close = close[0];
    let mut atr_sum = high[0] - low[0];
    for i in 1..period {
        let hl = high[i] - low[i];
        let hpc = (high[i] - prev_close).abs();
        let lpc = (low[i] - prev_close).abs();
        atr_sum += hl.max(hpc).max(lpc);
        prev_close = close[i];
    }
    let mut atr_mean = atr_sum * inv_p;

    let mut adx_acc = calc_dx(dmup, dmdown);
    let mut adx_state = f64::NAN;

    for i in period..n {
        let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
        dmup = dmup.mul_add(mult, dp);
        dmdown = dmdown.mul_add(mult, dm);

        let hl = high[i] - low[i];
        let hpc = (high[i] - prev_close).abs();
        let lpc = (low[i] - prev_close).abs();
        let tr = hl.max(hpc).max(lpc);
        prev_close = close[i];
        atr_mean = atr_mean.mul_add(mult, tr * inv_p);

        let atr_inv = di_scale / atr_mean;
        let pdi = (dmup * atr_inv).max(0.0);
        let mdi = (dmdown * atr_inv).max(0.0);
        let dx = calc_dx(dmup, dmdown);

        let kp = i - period;
        unsafe {
            *row0.add(kp) = dmup;
            *row1.add(kp) = dmdown;
            *row2.add(kp) = pdi;
            *row3.add(kp) = mdi;
            *row4.add(kp) = dx;
        }

        if i < adx_lead {
            adx_acc += dx;
            if i + 1 == adx_lead {
                adx_state = adx_acc * inv_p;
            }
        } else {
            if !adx_state.is_finite() {
                adx_state = adx_acc * inv_p;
            }
            adx_state = adx_state.mul_add(mult, dx * inv_p);
            let ka = i - adx_lead;
            unsafe { *row5.add(ka) = adx_state };
        }
    }

    vec![
        crate::cse::query::RowRef::window(&arena, off0, szp),
        crate::cse::query::RowRef::window(&arena, off1, szp),
        crate::cse::query::RowRef::window(&arena, off2, szp),
        crate::cse::query::RowRef::window(&arena, off3, szp),
        crate::cse::query::RowRef::window(&arena, off4, szp),
        crate::cse::query::RowRef::window(&arena, off5, sza),
    ]
}

/// Probe-only specialized 7-indicator fused hub kernel candidate (DM+DI+DX+ADX+ATR+Natr+Supertrend).
///
/// Uses hand-like recurrences for the directional movement + volatility + trend hub while
/// preserving CSE sink ordering and `RowRef` semantics. Feature-gated (`cse_kernel_probe`) and
/// intentionally not used by production routing.
///
/// Correctness-locked: if the candidate cannot be proven equivalent to
/// `dm_di_dx_adx_atr_natr_supertrend_shared_p` on first invocation, falls back to generated baseline.
/// Implements one-time validation using static OnceLock to lock mode (fast or fallback) based on max-abs error.
///
/// # Behavior
/// - First call: computes candidate full outputs + baseline from the generated kernel.
///   Evaluates sink mappings and validates all 7 rows. If best error <= tolerance, locks mode to fast;
///   else locks to fallback.
/// - Subsequent calls: if fast mode locked, runs candidate path only; if locked false,
///   calls `dm_di_dx_adx_atr_natr_supertrend_shared_p` directly.
/// - Edge cases (insufficient streams, n==0, period/lead anomalies, sink count not 7)
///   => fallback to generated baseline.
pub fn probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_fast(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct LockedMode {
        use_fast: bool,
        extra_perm: u8,
    }

    const PERMS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];

    #[inline(always)]
    fn calc_dp_dm(prev_high: &mut f64, prev_low: &mut f64, high: f64, low: f64) -> (f64, f64) {
        let mut dp = high - *prev_high;
        let mut dm = *prev_low - low;
        *prev_high = high;
        *prev_low = low;

        if dp < 0.0 {
            dp = 0.0;
        } else if dp > dm {
            dm = 0.0;
        }
        if dm < 0.0 {
            dm = 0.0;
        } else if dm > dp {
            dp = 0.0;
        }
        (dp, dm)
    }

    #[inline(always)]
    fn calc_dx(dmup: f64, dmdown: f64) -> f64 {
        let diff = (dmup - dmdown).abs();
        let sum = dmup + dmdown;
        (diff * 100.0 / sum).max(0.0)
    }

    static LOCKED_MODE: std::sync::OnceLock<LockedMode> = std::sync::OnceLock::new();
    const TOLERANCE: f64 = 1e-10;

    if streams.len() < 3 {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    let (high, low, close) = (streams[0], streams[1], streams[2]);
    let n = high.len().min(low.len()).min(close.len());

    let sink_count = SIG_probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base.n_sinks as usize;

    if sink_count == 0 || vals.sink_want.len() < sink_count || vals.sink_lead.len() < sink_count {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![crate::cse::query::RowRef::window(&arena, 0, 0); sink_count];
    }

    if high.len() != n || low.len() != n || close.len() != n {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    if sink_count < 15 {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    let period = vals.sink_lead[0] as usize;
    if period == 0 {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }
    let adx_lead = period.saturating_mul(2).saturating_sub(1);

    // Fixed core rows for DM/DI/DX/ADX.
    if !(vals.sink_want[0]
        && vals.sink_want[1]
        && vals.sink_want[2]
        && vals.sink_want[3]
        && vals.sink_want[4]
        && vals.sink_want[5])
    {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }
    if vals.sink_lead[1] as usize != period
        || vals.sink_lead[2] as usize != period
        || vals.sink_lead[3] as usize != period
        || vals.sink_lead[4] as usize != period
        || vals.sink_lead[5] as usize != adx_lead
    {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    // Expected extra wanted rows (atr/natr/supertrend primaries): 3 rows, all lead=period.
    let mut extra_rows = [usize::MAX; 3];
    let mut extra_count = 0usize;
    for row in 6..sink_count {
        if vals.sink_want[row] {
            if extra_count >= 3 || vals.sink_lead[row] as usize != period {
                return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
            }
            extra_rows[extra_count] = row;
            extra_count += 1;
        }
    }
    if extra_count != 3 {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    // Probe-only hub row uses SuperTrend step=3.0.
    let step = 3.0_f64;

    let mode = *LOCKED_MODE.get_or_init(|| {
        let baseline = probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
        if baseline.len() != sink_count {
            return LockedMode {
                use_fast: false,
                extra_perm: 0,
            };
        }

        let szp = n.saturating_sub(period);
        let sza = n.saturating_sub(adx_lead);
        for row in 0..sink_count {
            if !vals.sink_want[row] {
                continue;
            }
            let expect = if row == 5 {
                sza
            } else if row == 0
                || row == 1
                || row == 2
                || row == 3
                || row == 4
                || row == extra_rows[0]
                || row == extra_rows[1]
                || row == extra_rows[2]
            {
                szp
            } else {
                0
            };
            if baseline[row].len() != expect {
                return LockedMode {
                    use_fast: false,
                    extra_perm: 0,
                };
            }
        }

        let mult = (period - 1) as f64 / period as f64;
        let inv_p = 1.0 / period as f64;
        let di_scale = 100.0 * inv_p;

        let mut dmup = 0.0_f64;
        let mut dmdown = 0.0_f64;
        let mut prev_high = high[0];
        let mut prev_low = low[0];
        for i in 1..period {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup += dp;
            dmdown += dm;
        }

        let mut prev_close = close[0];
        let mut atr_sum = high[0] - low[0];
        for i in 1..period {
            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            atr_sum += hl.max(hpc).max(lpc);
            prev_close = close[i];
        }
        let mut atr_mean = atr_sum * inv_p;

        // SuperTrend init at bar period-1.
        let mut prev_st = 0.0_f64;
        let mut prev_lb = 0.0_f64;
        let mut prev_ub = 0.0_f64;
        let mut trend = false;
        {
            let i = period - 1;
            let med = 0.5 * (high[i] + low[i]);
            let mut ub = med + step * atr_mean;
            let mut lb = med - step * atr_mean;
            let crosses_up = close[i] > prev_st;
            let crosses_down = close[i] < prev_st;
            trend = crosses_up | (trend & !crosses_down);
            let st = if trend {
                lb = prev_lb.max(lb);
                lb
            } else {
                ub = prev_ub.min(ub);
                ub
            };
            prev_lb = lb;
            prev_ub = ub;
            prev_st = st;
        }

        let mut adx_acc = calc_dx(dmup, dmdown);
        let mut adx_state = f64::NAN;
        let mut perm_err = [0.0_f64; 6];

        for i in period..n {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup = dmup.mul_add(mult, dp);
            dmdown = dmdown.mul_add(mult, dm);

            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            let tr = hl.max(hpc).max(lpc);
            prev_close = close[i];
            atr_mean = atr_mean.mul_add(mult, tr * inv_p);

            let atr_inv = di_scale / atr_mean;
            let pdi = (dmup * atr_inv).max(0.0);
            let mdi = (dmdown * atr_inv).max(0.0);
            let dx = calc_dx(dmup, dmdown);

            let med = 0.5 * (high[i] + low[i]);
            let mut ub = med + step * atr_mean;
            let mut lb = med - step * atr_mean;
            let crosses_up = close[i] > prev_st;
            let crosses_down = close[i] < prev_st;
            trend = crosses_up | (trend & !crosses_down);
            let st = if trend {
                lb = prev_lb.max(lb);
                lb
            } else {
                ub = prev_ub.min(ub);
                ub
            };
            prev_lb = lb;
            prev_ub = ub;
            prev_st = st;

            let kp = i - period;
            let fixed = [dmup, dmdown, pdi, mdi, dx];
            for row in 0..5usize {
                let b = baseline[row][kp];
                let a = fixed[row];
                if a.is_finite() && b.is_finite() {
                    perm_err[0] = perm_err[0].max((a - b).abs());
                    perm_err[1] = perm_err[1].max((a - b).abs());
                    perm_err[2] = perm_err[2].max((a - b).abs());
                    perm_err[3] = perm_err[3].max((a - b).abs());
                    perm_err[4] = perm_err[4].max((a - b).abs());
                    perm_err[5] = perm_err[5].max((a - b).abs());
                }
            }

            if i < adx_lead {
                adx_acc += dx;
                if i + 1 == adx_lead {
                    adx_state = adx_acc * inv_p;
                }
            } else {
                if !adx_state.is_finite() {
                    adx_state = adx_acc * inv_p;
                }
                adx_state = adx_state.mul_add(mult, dx * inv_p);
                let ka = i - adx_lead;
                let b = baseline[5][ka];
                if adx_state.is_finite() && b.is_finite() {
                    for p in &mut perm_err {
                        *p = p.max((adx_state - b).abs());
                    }
                }
            }

            let extras = [atr_mean, (atr_mean / close[i]) * 100.0, st];
            for pidx in 0..6usize {
                let perm = PERMS[pidx];
                for j in 0..3usize {
                    let row = extra_rows[j];
                    let a = extras[perm[j]];
                    let b = baseline[row][kp];
                    if a.is_finite() && b.is_finite() {
                        perm_err[pidx] = perm_err[pidx].max((a - b).abs());
                    }
                }
            }
        }

        let mut best_idx = 0usize;
        let mut best_err = perm_err[0];
        for (i, e) in perm_err.iter().enumerate().skip(1) {
            if *e < best_err {
                best_err = *e;
                best_idx = i;
            }
        }

        let locked = LockedMode {
            use_fast: best_err <= TOLERANCE,
            extra_perm: best_idx as u8,
        };

        eprintln!(
            "probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_fast_mode,use_fast={},perm_idx={},err={:.12},tolerance={:.12}",
            locked.use_fast,
            locked.extra_perm,
            best_err,
            TOLERANCE,
        );

        locked
    });

    if !mode.use_fast {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    const MAX_SINKS: usize = 32;
    if sink_count > MAX_SINKS {
        return probe_dm_di_dx_adx_atr_natr_supertrend_shared_p_base(vals, streams);
    }

    let mut sizes = [0usize; MAX_SINKS];
    let mut offsets = [0usize; MAX_SINKS];
    let mut total = 0usize;
    for row in 0..sink_count {
        offsets[row] = total;
        if vals.sink_want[row] {
            let lead = vals.sink_lead[row] as usize;
            let sz = n.saturating_sub(lead);
            sizes[row] = sz;
            total += sz;
        }
    }

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
    let base: *mut f64 = if total == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    let row0 = unsafe { base.add(offsets[0]) };
    let row1 = unsafe { base.add(offsets[1]) };
    let row2 = unsafe { base.add(offsets[2]) };
    let row3 = unsafe { base.add(offsets[3]) };
    let row4 = unsafe { base.add(offsets[4]) };
    let row5 = unsafe { base.add(offsets[5]) };

    let perm = PERMS[mode.extra_perm as usize];
    let mut src_to_row = [usize::MAX; 3];
    for j in 0..3usize {
        src_to_row[perm[j]] = extra_rows[j];
    }
    let row_atr = unsafe { base.add(offsets[src_to_row[0]]) };
    let row_natr = unsafe { base.add(offsets[src_to_row[1]]) };
    let row_st = unsafe { base.add(offsets[src_to_row[2]]) };

    let mult = (period - 1) as f64 / period as f64;
    let inv_p = 1.0 / period as f64;
    let di_scale = 100.0 * inv_p;

    let hptr = high.as_ptr();
    let lptr = low.as_ptr();
    let cptr = close.as_ptr();

    let mut dmup = 0.0_f64;
    let mut dmdown = 0.0_f64;
    let mut prev_high = unsafe { *hptr };
    let mut prev_low = unsafe { *lptr };
    for i in 1..period {
        let hi = unsafe { *hptr.add(i) };
        let lo = unsafe { *lptr.add(i) };
        let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, hi, lo);
        dmup += dp;
        dmdown += dm;
    }

    let mut prev_close = unsafe { *cptr };
    let mut atr_sum = unsafe { *hptr - *lptr };
    for i in 1..period {
        let hi = unsafe { *hptr.add(i) };
        let lo = unsafe { *lptr.add(i) };
        let hl = hi - lo;
        let hpc = (hi - prev_close).abs();
        let lpc = (lo - prev_close).abs();
        atr_sum += hl.max(hpc).max(lpc);
        prev_close = unsafe { *cptr.add(i) };
    }
    let mut atr_mean = atr_sum * inv_p;

    let mut prev_st = 0.0_f64;
    let mut prev_lb = 0.0_f64;
    let mut prev_ub = 0.0_f64;
    let mut trend = false;
    {
        let i = period - 1;
        let hi = unsafe { *hptr.add(i) };
        let lo = unsafe { *lptr.add(i) };
        let ci = unsafe { *cptr.add(i) };
        let med = 0.5 * (hi + lo);
        let mut ub = med + step * atr_mean;
        let mut lb = med - step * atr_mean;
        let crosses_up = ci > prev_st;
        let crosses_down = ci < prev_st;
        trend = crosses_up | (trend & !crosses_down);
        let st = if trend {
            lb = prev_lb.max(lb);
            lb
        } else {
            ub = prev_ub.min(ub);
            ub
        };
        prev_lb = lb;
        prev_ub = ub;
        prev_st = st;
    }

    let mut adx_acc = calc_dx(dmup, dmdown);
    let mut adx_state = f64::NAN;

    for i in period..n {
        let hi = unsafe { *hptr.add(i) };
        let lo = unsafe { *lptr.add(i) };
        let ci = unsafe { *cptr.add(i) };

        let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, hi, lo);
        dmup = dmup.mul_add(mult, dp);
        dmdown = dmdown.mul_add(mult, dm);

        let hl = hi - lo;
        let hpc = (hi - prev_close).abs();
        let lpc = (lo - prev_close).abs();
        let tr = hl.max(hpc).max(lpc);
        prev_close = ci;
        atr_mean = atr_mean.mul_add(mult, tr * inv_p);

        let atr_inv = di_scale / atr_mean;
        let pdi = (dmup * atr_inv).max(0.0);
        let mdi = (dmdown * atr_inv).max(0.0);
        let dx = calc_dx(dmup, dmdown);

        let med = 0.5 * (hi + lo);
        let mut ub = med + step * atr_mean;
        let mut lb = med - step * atr_mean;
        let crosses_up = ci > prev_st;
        let crosses_down = ci < prev_st;
        trend = crosses_up | (trend & !crosses_down);
        let st = if trend {
            lb = prev_lb.max(lb);
            lb
        } else {
            ub = prev_ub.min(ub);
            ub
        };
        prev_lb = lb;
        prev_ub = ub;
        prev_st = st;

        let kp = i - period;
        unsafe {
            *row0.add(kp) = dmup;
            *row1.add(kp) = dmdown;
            *row2.add(kp) = pdi;
            *row3.add(kp) = mdi;
            *row4.add(kp) = dx;
            *row_atr.add(kp) = atr_mean;
            *row_natr.add(kp) = (atr_mean / ci) * 100.0;
            *row_st.add(kp) = st;
        }

        if i < adx_lead {
            adx_acc += dx;
            if i + 1 == adx_lead {
                adx_state = adx_acc * inv_p;
            }
        } else {
            adx_state = adx_state.mul_add(mult, dx * inv_p);
            let ka = i - adx_lead;
            unsafe { *row5.add(ka) = adx_state };
        }
    }

    let mut out = Vec::with_capacity(sink_count);
    for row in 0..sink_count {
        if vals.sink_want[row] {
            out.push(crate::cse::query::RowRef::window(
                &arena,
                offsets[row],
                sizes[row],
            ));
        } else {
            out.push(crate::cse::query::RowRef::window(&arena, 0, 0));
        }
    }
    out
}

/// Probe-only specialized DI kernel candidate with correctness-locked fast path.
///
/// Uses a hand-like DI recurrence that computes one reciprocal per bar for both
/// DI+ and DI- (reciprocal/materialization reduction intent). Correctness-locked:
/// if the candidate cannot be proven equivalent to `probe_di_n` on first invocation,
/// falls back to generated `probe_di_n`. Implements one-time validation using static
/// OnceLock/Once semantics to lock mode (fast or fallback) based on max-abs error.
///
/// # Behavior
/// - First call: computes candidate full outputs + baseline from probe_di_n(vals, streams).
///   Evaluates both sink mappings (row0=plus,row1=minus and swapped). Chooses mapping
///   with lower max-abs error over finite values on wanted rows. If best error <= tolerance,
///   locks mode to fast; else locks to fallback.
/// - Subsequent calls: if fast mode locked, runs candidate path only; if locked false,
///   calls probe_di_n directly.
/// - Edge cases (insufficient streams, n==0, period/lead anomalies, sink count not 2)
///   => fallback to probe_di_n.
pub fn probe_di_n_fast_recip_locked(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct LockedMode {
        use_fast: bool,
        swapped: bool,
    }

    fn compute_candidate(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
    ) -> (Vec<f64>, Vec<f64>) {
        let n = high.len().min(low.len()).min(close.len());
        let mut plus = vec![f64::NAN; n];
        let mut minus = vec![f64::NAN; n];

        if period == 0 || n <= period {
            return (plus, minus);
        }

        #[inline(always)]
        fn calc_dp_dm(prev_high: &mut f64, prev_low: &mut f64, high: f64, low: f64) -> (f64, f64) {
            let mut dp = high - *prev_high;
            let mut dm = *prev_low - low;
            *prev_high = high;
            *prev_low = low;

            if dp < 0.0 {
                dp = 0.0;
            } else if dp > dm {
                dm = 0.0;
            }
            if dm < 0.0 {
                dm = 0.0;
            } else if dm > dp {
                dp = 0.0;
            }
            (dp, dm)
        }

        let mult = (period - 1) as f64 / period as f64;
        let inv_p = 1.0 / period as f64;
        let scale = 100.0 * inv_p;

        // Match dm::State::init_state semantics exactly:
        // seed over bars [1, period), then steady update starts at `period`.
        let mut dmup = 0.0_f64;
        let mut dmdown = 0.0_f64;
        let mut prev_high = high[0];
        let mut prev_low = low[0];
        for i in 1..period {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup += dp;
            dmdown += dm;
        }

        // Match atr::State::init_state(..., composite=false) used by di recipe
        // via shared mean ATR chain:
        // mean seed over bars [0, period), then steady mean update starts at `period`.
        let mut prev_close = close[0];
        let mut atr_sum = high[0] - low[0];
        for i in 1..period {
            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            atr_sum += hl.max(hpc).max(lpc);
            prev_close = close[i];
        }
        let mut atr_mean = atr_sum * inv_p;

        // First published bar is `period` (lead = period), same as generated DI.
        for i in period..n {
            let (dp, dm) = calc_dp_dm(&mut prev_high, &mut prev_low, high[i], low[i]);
            dmup = dmup.mul_add(mult, dp);
            dmdown = dmdown.mul_add(mult, dm);

            let hl = high[i] - low[i];
            let hpc = (high[i] - prev_close).abs();
            let lpc = (low[i] - prev_close).abs();
            let tr = hl.max(hpc).max(lpc);
            prev_close = close[i];
            atr_mean = atr_mean.mul_add(mult, tr * inv_p);

            // One reciprocal shared by both DI legs; clamp with max(0) to mirror
            // recipe semantics (`Node::Max` with zero).
            let atr_inv = scale / atr_mean;
            plus[i] = (dmup * atr_inv).max(0.0);
            minus[i] = (dmdown * atr_inv).max(0.0);
        }

        (plus, minus)
    }

    fn max_abs_error_for_mapping(
        vals: &crate::cse::Values,
        baseline: &[crate::cse::query::RowRef],
        plus: &[f64],
        minus: &[f64],
        swapped: bool,
    ) -> f64 {
        let mut max_err = 0.0_f64;
        for row_idx in 0..2usize {
            if !vals.sink_want[row_idx] {
                continue;
            }
            let src = if (!swapped && row_idx == 0) || (swapped && row_idx == 1) {
                plus
            } else {
                minus
            };
            let lead = vals.sink_lead[row_idx] as usize;
            for (k, b) in baseline[row_idx].iter().enumerate() {
                let bar = lead + k;
                if bar >= src.len() {
                    break;
                }
                let a = src[bar];
                if a.is_finite() && b.is_finite() {
                    max_err = max_err.max((a - b).abs());
                }
            }
        }
        max_err
    }

    static LOCKED_MODE: std::sync::OnceLock<LockedMode> = std::sync::OnceLock::new();
    const TOLERANCE: f64 = 1e-10;

    if streams.len() < 3 {
        return probe_di_n(vals, streams);
    }

    let (high, low, close) = (streams[0], streams[1], streams[2]);
    let n = high.len().min(low.len()).min(close.len());

    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
        ];
    }

    if high.len() != n || low.len() != n || close.len() != n {
        return probe_di_n(vals, streams);
    }

    if (SIG_probe_di_n.n_sinks as usize) < 2 {
        return probe_di_n(vals, streams);
    }

    let lead0 = vals.sink_lead[0] as usize;
    let lead1 = vals.sink_lead[1] as usize;
    if lead0 == 0 || lead0 != lead1 {
        return probe_di_n(vals, streams);
    }
    let period = lead0;

    let want0 = vals.sink_want[0];
    let want1 = vals.sink_want[1];
    if !want0 && !want1 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            crate::cse::query::RowRef::window(&arena, 0, 0),
            crate::cse::query::RowRef::window(&arena, 0, 0),
        ];
    }

    let mode = *LOCKED_MODE.get_or_init(|| {
        let baseline = probe_di_n(vals, streams);
        if baseline.len() < 2 {
            return LockedMode {
                use_fast: false,
                swapped: false,
            };
        }

        let (plus, minus) = compute_candidate(high, low, close, period);
        let err_normal = max_abs_error_for_mapping(vals, &baseline, &plus, &minus, false);
        let err_swapped = max_abs_error_for_mapping(vals, &baseline, &plus, &minus, true);

        let locked = if err_normal <= err_swapped {
            LockedMode {
                use_fast: err_normal <= TOLERANCE,
                swapped: false,
            }
        } else {
            LockedMode {
                use_fast: err_swapped <= TOLERANCE,
                swapped: true,
            }
        };

        eprintln!(
            "probe_di_n_fast_recip_locked_mode,use_fast={},swapped={},err_normal={:.12},err_swapped={:.12},tolerance={:.12}",
            locked.use_fast,
            locked.swapped,
            err_normal,
            err_swapped,
            TOLERANCE,
        );

        locked
    });

    if !mode.use_fast {
        return probe_di_n(vals, streams);
    }

    let (plus, minus) = compute_candidate(high, low, close, period);

    let rsz0 = n.saturating_sub(lead0);
    let rsz1 = n.saturating_sub(lead1);
    let off0 = 0usize;
    let off1 = rsz0;
    let total = rsz0 + rsz1;

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
    let base: *mut f64 = if total == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    let src0 = if mode.swapped { &minus } else { &plus };
    let src1 = if mode.swapped { &plus } else { &minus };

    if want0 {
        for k in 0..rsz0 {
            let bar = lead0 + k;
            unsafe { *base.add(off0 + k) = src0[bar] };
        }
    }
    if want1 {
        for k in 0..rsz1 {
            let bar = lead1 + k;
            unsafe { *base.add(off1 + k) = src1[bar] };
        }
    }

    vec![
        if want0 {
            crate::cse::query::RowRef::window(&arena, off0, rsz0)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
        if want1 {
            crate::cse::query::RowRef::window(&arena, off1, rsz1)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        },
    ]
}

/// Probe-only specialized CMO kernel candidate.
///
/// Uses CMO's rolling up/down recurrence directly (same arithmetic shape as the
/// hand implementation) while preserving CSE sink ordering and `RowRef`
/// semantics. Feature-gated (`cse_kernel_probe`) and intentionally not used by
/// production routing.
pub fn probe_cmo_n_fast(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    let n = streams.iter().map(|s| s.len()).min().unwrap_or(0);
    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![crate::cse::query::RowRef::window(&arena, 0, 0)];
    }

    let lead = vals.sink_lead[0] as usize;
    let want = vals.sink_want[0];
    let rsz0 = n.saturating_sub(lead);

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(rsz0);
    let base: *mut f64 = if rsz0 == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    if rsz0 == 0 {
        return vec![if want {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        }];
    }

    let real = streams[0];
    let period = lead.saturating_sub(1);
    if period == 0 {
        return probe_cmo_n(vals, streams);
    }

    let mut up_sum = 0.0_f64;
    let mut down_sum = 0.0_f64;

    for i in 1..=period {
        let diff = real[i] - real[i - 1];
        up_sum += diff.max(0.0);
        down_sum += (-diff).max(0.0);
    }

    if want {
        for t in lead..n {
            let diff = real[t] - real[t - 1];
            let up = diff.max(0.0);
            let down = (-diff).max(0.0);

            let drop_i = t - period;
            let old_diff = real[drop_i] - real[drop_i - 1];
            let old_up = old_diff.max(0.0);
            let old_down = (-old_diff).max(0.0);

            up_sum += up - old_up;
            down_sum += down - old_down;

            unsafe {
                *base.add(t - lead) = 100.0 * (up_sum - down_sum) / (up_sum + down_sum);
            }
        }
    }

    vec![if want {
        crate::cse::query::RowRef::window(&arena, 0, rsz0)
    } else {
        crate::cse::query::RowRef::window(&arena, 0, 0)
    }]
}

/// Probe-only specialized STOCH kernel candidate.
///
/// Keeps CSE recipe semantics (same extrema finder family and smoothing
/// arithmetic) but uses a hand-like local state layout to estimate the generic
/// scalar-kernel state/indexing tax on singleton `stoch_kd`.
pub fn probe_stoch_kd_fast(
    vals: &crate::cse::Values,
    streams: &[&[f64]],
) -> Vec<crate::cse::query::RowRef> {
    use crate::cse::signature::{K_ROLLMAX, K_ROLLMIN, K_ROLLSUM};
    use crate::indicators::{max::find_max_simd, min::find_min_simd};
    use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    let ns = SIG_probe_stoch_kd.n_sinks as usize;
    if streams.len() < 3 {
        return probe_stoch_kd(vals, streams);
    }

    let (high, low, close) = (streams[0], streams[1], streams[2]);
    let n = high.len().min(low.len()).min(close.len());
    if n == 0 {
        let arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(0usize);
        let arena: Arc<[f64]> = unsafe { arc.assume_init() };
        return (0..ns)
            .map(|_| crate::cse::query::RowRef::window(&arena, 0, 0))
            .collect();
    }

    // This probe targets the common run-only shape where all three streams are
    // lockstep. Keep odd stream-shape handling in generated fallback.
    if high.len() != n || low.len() != n || close.len() != n {
        return probe_stoch_kd(vals, streams);
    }

    // Derive periods from the erased signature + values instead of hard-coding.
    let mut k_period = 0usize;
    let mut rollsum_periods: Vec<usize> = Vec::new();
    for k in 0..(SIG_probe_stoch_kd.len as usize) {
        let kind = SIG_probe_stoch_kd.kinds[k];
        if kind == K_ROLLMAX || kind == K_ROLLMIN {
            k_period = k_period.max(vals.periods[k] as usize);
        }
        if kind == K_ROLLSUM {
            let p = vals.periods[k] as usize;
            if p > 0 && !rollsum_periods.contains(&p) {
                rollsum_periods.push(p);
            }
        }
    }
    if k_period == 0 || rollsum_periods.is_empty() {
        return probe_stoch_kd(vals, streams);
    }
    let k_slow = rollsum_periods[0];
    let d_period = *rollsum_periods.get(1).unwrap_or(&k_slow);
    if k_slow == 0 || d_period == 0 {
        return probe_stoch_kd(vals, streams);
    }

    // Probe row layout mirrors generated scalar row layout: one contiguous arena,
    // per-sink windows based on sink_lead/sink_want.
    let mut rsz: Vec<usize> = Vec::with_capacity(ns);
    for s in 0..ns {
        rsz.push(n.saturating_sub(vals.sink_lead[s] as usize));
    }
    let mut off: Vec<usize> = Vec::with_capacity(ns);
    let mut total = 0usize;
    for &len in &rsz {
        off.push(total);
        total += len;
    }

    let mut arc: Arc<[MaybeUninit<f64>]> = Arc::new_uninit_slice(total);
    let base: *mut f64 = if total == 0 {
        std::ptr::null_mut()
    } else {
        Arc::get_mut(&mut arc)
            .expect("fresh arena is uniquely owned")
            .as_mut_ptr()
            .cast::<f64>()
    };
    let arena: Arc<[f64]> = unsafe { arc.assume_init() };

    // Hand-like local extrema state: direct value/trail carries, direct
    // `find_*_simd::<4>` rescan contract, no generic state arrays/indexing.
    let init_look_back = (k_period + 1).min(n);

    let mut min_val = low[0];
    let mut min_trail = 0usize;
    for &bar in low.iter().take(init_look_back).skip(1) {
        if bar <= min_val {
            min_val = bar;
            min_trail = 0;
        } else {
            min_trail += 1;
        }
    }

    let mut max_val = high[0];
    let mut max_trail = 0usize;
    for &bar in high.iter().take(init_look_back).skip(1) {
        if bar >= max_val {
            max_val = bar;
            max_trail = 0;
        } else {
            max_trail += 1;
        }
    }

    let look_back = k_period - 1;

    #[inline(always)]
    unsafe fn roll_min_probe(
        src: &[f64],
        i: usize,
        period: usize,
        look_back: usize,
        min_val: &mut f64,
        min_trail: &mut usize,
    ) -> f64 {
        *min_trail += 1;
        if period <= *min_trail {
            let search_start = i - look_back;
            let window = src.get_unchecked(search_start..=i);
            let (v, idx) = find_min_simd::<4>(window);
            *min_val = v;
            *min_trail = i - (search_start + idx);
        } else {
            let current = *src.get_unchecked(i);
            if current <= *min_val {
                *min_val = current;
                *min_trail = 0;
            }
        }
        *min_val
    }

    #[inline(always)]
    unsafe fn roll_max_probe(
        src: &[f64],
        i: usize,
        period: usize,
        look_back: usize,
        max_val: &mut f64,
        max_trail: &mut usize,
    ) -> f64 {
        *max_trail += 1;
        if period <= *max_trail {
            let search_start = i - look_back;
            let window = src.get_unchecked(search_start..=i);
            let (v, idx) = find_max_simd::<4>(window);
            *max_val = v;
            *max_trail = i - (search_start + idx);
        } else {
            let current = *src.get_unchecked(i);
            if current >= *max_val {
                *max_val = current;
                *max_trail = 0;
            }
        }
        *max_val
    }

    let mut prev_k = Buffer::new(k_slow);
    let mut prev_d = Buffer::new(d_period);
    let mut k_sum = 0.0_f64;
    let mut d_sum = 0.0_f64;
    let k_mult = 1.0 / k_slow as f64;
    let d_mult = 1.0 / d_period as f64;

    let init_start = k_period + 1;
    let init_end = k_period + k_slow + d_period;
    let mut start = init_start;

    for i in init_start..n.min(init_end) {
        let kfast = unsafe {
            let min = roll_min_probe(low, i, k_period, look_back, &mut min_val, &mut min_trail);
            let max = roll_max_probe(high, i, k_period, look_back, &mut max_val, &mut max_trail);
            100.0 * (*close.get_unchecked(i) - min) / (max - min).max(f64::EPSILON)
        };

        k_sum += kfast;
        if let Some(old_k) = prev_k.push_with_info(kfast) {
            k_sum -= old_k;
        }
        if prev_k.is_full() {
            let k = k_sum * k_mult;
            if ns >= 1 && vals.sink_want[0] && i >= vals.sink_lead[0] as usize {
                let p = unsafe { base.add(off[0] + (i - vals.sink_lead[0] as usize)) };
                unsafe { *p = k };
            }
            d_sum += k;
            prev_d.push(k);
        }
        start = i + 1;
    }

    for i in start..n {
        let kfast = unsafe {
            let min = roll_min_probe(low, i, k_period, look_back, &mut min_val, &mut min_trail);
            let max = roll_max_probe(high, i, k_period, look_back, &mut max_val, &mut max_trail);
            100.0 * (*close.get_unchecked(i) - min) / (max - min).max(f64::EPSILON)
        };

        let old_k = prev_k.push_with_info(kfast).unwrap_or(0.0);
        k_sum += kfast - old_k;
        let k = k_sum * k_mult;

        let old_d = prev_d.push_with_info(k).unwrap_or(0.0);
        d_sum += k - old_d;
        let d = d_sum * d_mult;

        if ns >= 1 && vals.sink_want[0] && i >= vals.sink_lead[0] as usize {
            let p = unsafe { base.add(off[0] + (i - vals.sink_lead[0] as usize)) };
            unsafe { *p = k };
        }
        if ns >= 2 && vals.sink_want[1] && i >= vals.sink_lead[1] as usize {
            let p = unsafe { base.add(off[1] + (i - vals.sink_lead[1] as usize)) };
            unsafe { *p = d };
        }
    }

    (0..ns)
        .map(|s| {
            if vals.sink_want[s] {
                crate::cse::query::RowRef::window(&arena, off[s], rsz[s])
            } else {
                crate::cse::query::RowRef::window(&arena, 0, 0)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        probe_ao_s_m, probe_ao_s_m_fast, probe_cmo_n, probe_cmo_n_fast, probe_stoch_kd,
        probe_stoch_kd_fast,
    };
    use crate::cse::{erase_request, Planable, Request};
    use crate::indicator_types::Indicator;
    use crate::indicators::ao::Ao;
    use crate::indicators::cmo::Cmo;
    use crate::indicators::stoch::Stoch;

    fn sample_hl(n: usize) -> (Vec<f64>, Vec<f64>) {
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as f64;
            let base = 100.0 + (x * 0.017).sin() * 3.0 + (x * 0.031).cos() * 1.5;
            let spread = 0.8 + (x * 0.013).sin().abs() * 0.6;
            high.push(base + spread);
            low.push(base - spread);
        }
        (high, low)
    }

    fn sample_close(n: usize) -> Vec<f64> {
        let mut close = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as f64;
            let v = 100.0 + (x * 0.021).sin() * 2.8 + (x * 0.037).cos() * 1.1 + x * 0.0002;
            close.push(v);
        }
        close
    }

    fn sample_hlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let (high, low) = sample_hl(n);
        let mut close = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as f64;
            let w = ((x * 0.019).sin() + 1.0) * 0.5;
            close.push(low[i] + (high[i] - low[i]) * w);
        }
        (high, low, close)
    }

    #[test]
    fn probe_ao_fast_matches_generated_primary_only() {
        let (high, low) = sample_hl(512);
        let streams: [&[f64]; 2] = [high.as_slice(), low.as_slice()];

        let req = Request::new().add(Ao::recipe(&[]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

        let a = probe_ao_s_m(&vals, &streams);
        let b = probe_ao_s_m_fast(&vals, &streams);

        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.len(), rb.len());
            assert_eq!(ra.as_slice(), rb.as_slice());
        }
    }

    #[test]
    fn probe_ao_fast_matches_generated_with_optionals() {
        let (high, low) = sample_hl(512);
        let streams: [&[f64]; 2] = [high.as_slice(), low.as_slice()];

        let req = Request::new().add(Ao::recipe(&[]), Some(&[true, true, true]));
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

        let a = probe_ao_s_m(&vals, &streams);
        let b = probe_ao_s_m_fast(&vals, &streams);

        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.len(), rb.len());
            assert_eq!(ra.as_slice(), rb.as_slice());
        }
    }

    #[test]
    fn probe_ao_fast_matches_hand_primary_row() {
        let (high, low) = sample_hl(1024);
        let streams: [&[f64]; 2] = [high.as_slice(), low.as_slice()];

        let req = Request::new().add(Ao::recipe(&[]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");
        let fast = probe_ao_s_m_fast(&vals, &streams);

        let (hand, _) = Ao::indicator(&streams, &[], None).expect("hand ao");

        assert_eq!(fast[0].as_slice(), hand[0].as_slice());
    }

    #[test]
    fn probe_cmo_fast_matches_generated_primary_only() {
        let close = sample_close(1024);
        let streams: [&[f64]; 1] = [close.as_slice()];

        let req = Request::new().add(Cmo::recipe(&[14.0]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

        let a = probe_cmo_n(&vals, &streams);
        let b = probe_cmo_n_fast(&vals, &streams);

        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.len(), rb.len());
            assert_eq!(ra.as_slice(), rb.as_slice());
        }
    }

    #[test]
    fn probe_cmo_fast_matches_generated_multiple_periods() {
        let close = sample_close(2048);
        let streams: [&[f64]; 1] = [close.as_slice()];

        for p in [5.0_f64, 14.0, 33.0, 55.0] {
            let req = Request::new().add(Cmo::recipe(&[p]), None);
            let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

            let a = probe_cmo_n(&vals, &streams);
            let b = probe_cmo_n_fast(&vals, &streams);

            assert_eq!(a.len(), b.len(), "row count mismatch at period {p}");
            for (ra, rb) in a.iter().zip(b.iter()) {
                assert_eq!(ra.len(), rb.len(), "len mismatch at period {p}");
                assert_eq!(ra.as_slice(), rb.as_slice(), "value mismatch at period {p}");
            }
        }
    }

    #[test]
    fn probe_cmo_fast_matches_hand_primary_row() {
        let close = sample_close(1024);
        let streams: [&[f64]; 1] = [close.as_slice()];

        let req = Request::new().add(Cmo::recipe(&[14.0]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");
        let fast = probe_cmo_n_fast(&vals, &streams);

        let (hand, _) = Cmo::indicator(&streams, &[14.0], None).expect("hand cmo");

        assert_eq!(fast[0].as_slice(), hand[0].as_slice());
    }

    #[test]
    fn probe_stoch_fast_matches_generated_primary_only() {
        let (high, low, close) = sample_hlc(2048);
        let streams: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];

        let req = Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

        let a = probe_stoch_kd(&vals, &streams);
        let b = probe_stoch_kd_fast(&vals, &streams);

        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.len(), rb.len());
            assert_eq!(ra.as_slice(), rb.as_slice());
        }
    }

    #[test]
    fn probe_stoch_fast_matches_generated_multiple_periods() {
        let (high, low, close) = sample_hlc(3072);
        let streams: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in [[14.0_f64, 3.0, 3.0], [21.0, 5.0, 3.0], [34.0, 5.0, 5.0]] {
            let req = Request::new().add(Stoch::recipe(&options), None);
            let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");

            let a = probe_stoch_kd(&vals, &streams);
            let b = probe_stoch_kd_fast(&vals, &streams);

            assert_eq!(
                a.len(),
                b.len(),
                "row count mismatch at options {:?}",
                options
            );
            for (ra, rb) in a.iter().zip(b.iter()) {
                assert_eq!(ra.len(), rb.len(), "len mismatch at options {:?}", options);
                assert_eq!(
                    ra.as_slice(),
                    rb.as_slice(),
                    "value mismatch at options {:?}",
                    options
                );
            }
        }
    }

    #[test]
    fn probe_stoch_fast_matches_hand_primary_rows() {
        let (high, low, close) = sample_hlc(2048);
        let streams: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];

        let req = Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None);
        let (_arena, _sig, vals, _owners) = erase_request(&req).expect("erase_request");
        let fast = probe_stoch_kd_fast(&vals, &streams);

        let (hand, _) = Stoch::indicator(&streams, &[14.0, 3.0, 3.0], None).expect("hand stoch");

        assert_eq!(fast[0].as_slice(), hand[0].as_slice());
        assert_eq!(fast[1].as_slice(), hand[1].as_slice());
    }
}
