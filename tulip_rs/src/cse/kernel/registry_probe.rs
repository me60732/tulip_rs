//! Probe-only mini kernel registry for focused code-shape inspection.
//!
//! Enabled only with `--features cse_kernel_probe`.
//! This module is intentionally not wired into production routing.

tulip_rs_macros::kernel! {
    // Singleton outliers under investigation.
    probe_ao_s_m: Ao(s, m);
    probe_di_n: Di(p);
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
