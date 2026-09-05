use crate::indicators::max::{find_max_scalar as find_remainder, State};
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::max::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::max::indicator_by_options;
pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;

use core::ops::Range;
pub(crate) const CHUNK_1: Range<usize> = 1..15;
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
    use super::{find_max_scalar, find_max_simd, TSimdState, TState, Warm, State};

    pub struct SimdState<const N: usize> {
        pub max: Simd<f64, N>,
        pub trail: Simd<usize, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [],
             scalar: [max, trail]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], usize, usize);
        type Outputs = (Simd<f64, N>, Simd<usize, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (real, i, look_back): Self::Inputs<'_>
        ) -> Self::Outputs {
            let current = crate::extract_simd_inputs_at_index!(i, N, val @ real);

            self.calc_w_current::<4>((real, i, look_back, current))
        }
    }
    impl<const N: usize> SimdState<N> {
        #[inline(always)]
        pub fn calc_chuncked<const WINDOW_LANES: usize>(
            &mut self,
            (real, i, look_back): ([*const f64; N], usize, usize)
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let current = crate::extract_simd_inputs_at_index!(i, N, val @ real);

            self.calc_w_current::<WINDOW_LANES>((real, i, look_back, current))
        }
        #[inline(always)]
        pub fn calc_w_current<const WINDOW_LANES: usize>(
            &mut self,
            (real, i, look_back, current): ([*const f64; N], usize, usize, Simd<f64, N>)
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let mut trail = self.trail;
            let mut max = self.max;

            let lookback_simd = Simd::splat(look_back);
            let needs_search = lookback_simd.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();
            let current_is_new_max = current.simd_ge(max);

            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
            max = current_is_new_max.select(current, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);

            if search_mask != 0 {
                let start = i - look_back;
                let take = look_back;

                let max_array = max.as_mut_array();
                let trail_array = trail.as_mut_array();
                let current = current.as_array();
                // Const loop - compiler will unroll this automatically
                let mut lane = 0;
                while lane < N {
                    if search_mask & (1 << lane) != 0 {
                        let window = unsafe { std::slice::from_raw_parts(real[lane].add(start), take) };
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
    use super::{find_max_scalar, find_max_simd, TSimdState, TState, State, Warm, CHUNK_1};

    pub struct SimdState<const N: usize> {
        pub max: Simd<f64, N>,
        pub trail: Simd<usize, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [],
             scalar: [max, trail]
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<usize, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (real, i, look_back): Self::Inputs<'_>
        ) -> Self::Outputs {
            let current = Simd::splat(unsafe { *real[0].add(i[0]) });
            self.calc_w_current((real, i, look_back, current))
        }
    }
    impl<const N: usize> SimdState<N> {
        #[inline(always)]
        pub fn calc_w_current(
            &mut self,
            (real, i, look_back, current): ([*const f64; N], Simd<usize, N>, Simd<usize, N>, Simd<f64, N>)
        ) -> (Simd<f64, N>, Simd<usize, N>) {
            let mut trail = self.trail;
            let mut max = self.max;

            let needs_search = look_back.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();
            let current_is_new_max = current.simd_ge(max);

            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
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
                        let take = look_back_array[lane];
                        let window = unsafe { std::slice::from_raw_parts(real[lane].add(start), take) };
                        let (max_val, max_idx) = if CHUNK_1.contains(&take) {
                            find_max_scalar(window, current[lane])
                        } else {
                            find_max_simd::<4>(window, current[lane])
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
    let mut global_max = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut max_idx = 0;
    let search_window = unsafe { window.get_unchecked(1..) };

    let mut best_values = Simd::<f64, N>::splat(0.0);
    let mut best_start = usize::MAX; // sentinel: no chunk has updated yet

    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::<f64, N>::from_slice(chunk);
        let mask = values.simd_ge(global_max);
        if mask.any() {
            global_max = Simd::splat(values.reduce_max());
            best_values = values; // save the chunk that holds the max
            best_start = chunk_idx;
        }
    }

    // Position finding done once outside the loop
    if best_start != usize::MAX {
        let i = if N <= 4 {
            best_values
                .simd_eq(global_max)
                .to_bitmask()
                .ilog2() as usize
        } else {
            let eq_mask = best_values.simd_eq(global_max);
            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }
            i
        };
        max_idx = best_start * N + 1 + i;
    }
    let mut global_max = global_max[0];
    let processed_len = (search_window.len() / N) * N;
    let remainder = unsafe { search_window.get_unchecked(processed_len..) };
    if !remainder.is_empty() {
        let (rem_max, rem_idx) = find_remainder(remainder);
        if rem_max >= global_max {
            global_max = rem_max;
            max_idx = processed_len + 1 + rem_idx;
        }
    }

    if global_max > current {
        return (global_max, max_idx);
    }
    (current, window.len())
}
