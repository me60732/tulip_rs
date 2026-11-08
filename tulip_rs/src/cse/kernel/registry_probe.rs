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

#[cfg(test)]
mod tests {
    use super::{probe_ao_s_m, probe_ao_s_m_fast, probe_cmo_n, probe_cmo_n_fast};
    use crate::cse::{erase_request, Planable, Request};
    use crate::indicator_types::Indicator;
    use crate::indicators::ao::Ao;
    use crate::indicators::cmo::Cmo;

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
}
