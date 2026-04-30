use crate::indicators::max::{find_max_scalar as find_remainder, State};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::max::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::max::indicator_by_options;

pub struct SimdState<const N: usize> {
    pub max: Simd<f64, N>,
    pub trail: Simd<usize, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut max = [0.0; N];
        let mut trail: [usize; N] = [0; N];

        for i in 0..N {
            max[i] = states[i].max;
            trail[i] = states[i].trail;
        }

        Self {
            max: Simd::from_array(max),
            trail: Simd::from_array(trail),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let max = self.max.to_array();
        let trail = self.trail.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(max[i], trail[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let max = self.max.to_array();
        let trail = self.trail.to_array();

        for i in 0..N {
            states[i].max = max[i];
            states[i].trail = trail[i];
        }
    }
}

pub(crate) use std::{
    f64,
    simd::{
        cmp::{SimdPartialEq, SimdPartialOrd},
        num::SimdFloat,
        Simd,
    },
};
mod import {
    pub(crate) use crate::indicators::simd_indicators::simd_types::UsizeConstants;
    pub(crate) use std::{
        f64,
        simd::{
            cmp::{SimdPartialEq, SimdPartialOrd},
            Select, Simd,
        },
    };
}
pub mod assets {
    use super::import::*;
    use super::{find_max_scalar, find_max_simd, SimdState};
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            real: [*const f64; N],
            i: usize,
            look_back: usize,
        ) -> (Simd<f64, N>, Simd<usize, N>);
        unsafe fn calc_unchecked_simd_w_current<const WINDOW_LANES: usize>(
            &mut self,
            real: [*const f64; N],
            i: usize,
            look_back: usize,
            current: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>);
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            real: [*const f64; N],
            i: usize,
            look_back: usize,
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let current = crate::extract_simd_inputs_at_index!(i, N, val @ real);

            self.calc_unchecked_simd_w_current::<WINDOW_LANES>(real, i, look_back, current)
        }
        #[inline(always)]
        unsafe fn calc_unchecked_simd_w_current<const WINDOW_LANES: usize>(
            &mut self,
            real: [*const f64; N],
            i: usize,
            look_back: usize,
            current: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let mut trail = self.trail;
            let mut max = self.max;

            let lookback_simd = Simd::splat(look_back);
            let needs_search = lookback_simd.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();

            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
            let current_is_new_max = current.simd_ge(max);

            max = current_is_new_max.select(current, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);

            if search_mask != 0 {
                let start = i - look_back;
                let take = i - start;

                let max_array = max.as_mut_array();
                let trail_array = trail.as_mut_array();
                let current = current.as_array();
                // Const loop - compiler will unroll this automatically
                let mut lane = 0;
                while lane < N {
                    if search_mask & (1 << lane) != 0 {
                        let window = std::slice::from_raw_parts(real[lane].add(start), take);
                        let (max_val, max_idx) = if WINDOW_LANES == 1 {
                            find_max_scalar(window, current[lane])
                        } else {
                            find_max_simd::<WINDOW_LANES>(window, current[lane])
                        };
                        max_array[lane] = max_val;
                        trail_array[lane] = take - max_idx;
                    }
                    lane += 1;
                }
            }

            self.max = max;
            self.trail = trail;
            (max, trail)
        }
    }
}
pub mod options {
    use super::import::*;
    use super::{find_max_scalar, find_max_simd, SimdState};
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            &mut self,
            real: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>);
        unsafe fn calc_unchecked_simd_w_current(
            &mut self,
            real: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
            current: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>);
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            real: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let current = Simd::splat(*real[0].add(i[0]));
            self.calc_unchecked_simd_w_current(real, i, look_back, current)
        }
        #[inline(always)]
        unsafe fn calc_unchecked_simd_w_current(
            &mut self,
            real: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
            current: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let mut trail = self.trail;
            let mut max = self.max;

            let needs_search = look_back.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();

            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
            let current_is_new_max = current.simd_ge(max);

            max = current_is_new_max.select(current, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);

            if search_mask != 0 {
                let i_array = i.as_array();
                let look_back_array = look_back.as_array();
                let max_array = max.as_mut_array();
                let trail_array = trail.as_mut_array();
                let current = current.as_array();
                // Const loop - compiler will unroll this automatically
                let mut lane = 0;
                while lane < N {
                    if search_mask & (1 << lane) != 0 {
                        let start = i_array[lane] - look_back_array[lane];
                        let take = i_array[lane] - start;
                        let window = std::slice::from_raw_parts(real[lane].add(start), take);
                        let (max_val, max_idx) = if take < 14 {
                            find_max_scalar(window, current[lane])
                        } else {
                            find_max_simd::<8>(window, current[lane])
                        };
                        max_array[lane] = max_val;
                        trail_array[lane] = take - max_idx;
                    }
                    lane += 1;
                }
            }

            self.max = max;
            self.trail = trail;
            (max, trail)
        }
    }
}

#[inline(always)]
pub(crate) fn find_max_scalar(window: &[f64], current: f64) -> (f64, usize) {
    let end = window.len();
    let mut max_val = current;
    let mut max_idx = end;
    let mut i = end;

    while i > 0 {
        i -= 1;
        let val = unsafe { *window.get_unchecked(i) };
        if val > max_val {
            max_val = val;
            max_idx = i;
        }
    }

    (max_val, max_idx)
}

pub(crate) fn find_max_simd<const N: usize>(window: &[f64], current: f64) -> (f64, usize) {
    let mut global_max = unsafe { *window.get_unchecked(0) };
    //let mut global_max = unsafe { *window.get_unchecked(0) };
    let mut max_idx = 0; // Index for current
    let search_window = unsafe { window.get_unchecked(1..) };
    // Process chunks with SIMD
    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::<f64, N>::from_slice(chunk);
        let mask = values.simd_ge(Simd::splat(global_max));

        if mask.any() {
            global_max = values.reduce_max();
            // Create equality mask for the new maximum
            let eq_mask = values.simd_eq(Simd::splat(global_max));

            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }

            max_idx = chunk_idx * N + i + 1;
        }
    }

    // Handle remainder elements
    let processed_len = (search_window.len() / N) * N;
    let remainder = unsafe { search_window.get_unchecked(processed_len..) };
    if !remainder.is_empty() {
        let (rem_max, rem_idx) = find_remainder(remainder);
        if rem_max >= global_max {
            global_max = rem_max;
            max_idx = processed_len + 1 + rem_idx; // +1 for search_window offset
        }
    }
    if global_max > current {
        return (global_max, max_idx);
    }
    (current, window.len())
}
