#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::min::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::min::indicator_by_options;

use crate::indicators::min::{find_min_scalar as find_remainder,  State};
pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;

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
    pub(crate) use std::simd::Select;
}
pub mod assets {
    //! Per-asset road SIMD helpers for the Rolling Minimum indicator.
    use super::import::*;
    use super::*;

    pub struct SimdState<const N: usize> {
        pub min: Simd<f64, N>,
        pub trail: Simd<usize, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
            sub: [],
            scalar: [min, trail]
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
            let mut min = self.min;

            let lookback_simd = Simd::splat(look_back);
            let needs_search = lookback_simd.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();
            let current_is_new_min = current.simd_le(min);
            
            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
            min = current_is_new_min.select(current, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);

            if search_mask != 0 {
                let start = i - look_back;
                let take = look_back;

                let min_array = min.as_mut_array();
                let trail_array = trail.as_mut_array();
                let current = current.as_array();
                
                let mut lane = 0;
                while lane < N {
                    if search_mask & (1 << lane) != 0 {
                        let window = unsafe { std::slice::from_raw_parts(real[lane].add(start), take) };
                        let (min_val, min_idx) = if WINDOW_LANES == 1 {
                            find_min_scalar(window, current[lane])
                        } else {
                            find_min_simd::<WINDOW_LANES>(window, current[lane])
                        };

                        min_array[lane] = min_val;
                        trail_array[lane] = take - min_idx;
                    }
                    lane += 1;
                }
            }

            self.min = min;
            self.trail = trail;
            (min, trail)
        }
    }
}
pub mod options {
    //! Per-option road SIMD helpers for the Rolling Minimum indicator.
    use crate::indicators::simd_indicators::max_simd::CHUNK_1;

use super::import::*;
    use super::*;
    
    pub struct SimdState<const N: usize> {
        pub min: Simd<f64, N>,
        pub trail: Simd<usize, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
            sub: [],
            scalar: [min, trail]
        );
    }
   
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<usize, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (real, i, look_back): Self::Inputs<'_>,
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
            let mut min = self.min;

            let needs_search = look_back.simd_eq(trail);
            let search_mask = needs_search.to_bitmask();
            let current_is_new_min = current.simd_le(min);
            
            trail = needs_search.select(trail, trail + UsizeConstants::ONE);
            min = current_is_new_min.select(current, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);

            if search_mask != 0 {
                let look_back_array = look_back.as_array();
                let i_array = i.as_array();
                let min_array = min.as_mut_array();
                let trail_array = trail.as_mut_array();
                let current = current.as_array();
                // Const loop - compiler will unroll this automatically
                let mut lane = 0;
                while lane < N {
                    if search_mask & (1 << lane) != 0 {
                        let start = i_array[lane] - look_back_array[lane];
                        let take = look_back_array[lane];
                        let window = unsafe { std::slice::from_raw_parts(real[lane].add(start), take) };
                        let (min_val, min_idx) = if CHUNK_1.contains(&take) {
                            find_min_scalar(window, current[lane])
                        } else {
                            find_min_simd::<4>(window, current[lane])
                        };

                        min_array[lane] = min_val;
                        trail_array[lane] = take - min_idx;
                    }
                    lane += 1;
                }
            }

            self.min = min;
            self.trail = trail;
            (min, trail)
        }
    }
}

/// Scans `window` scalar-by-scalar to find the minimum value, also considering `current`.
///
/// Returns a tuple `(min_value, index_of_min)` where `index_of_min` is the position
/// within `window` (or `window.len()` if `current` is the minimum).
#[inline(always)]
pub(crate) fn find_min_scalar(window: &[f64], current: f64) -> (f64, usize) {
    let end = window.len();
    let mut min_val = current;
    let mut min_idx = end;
    let mut i = end;

    while i > 0 {
        i -= 1;
        let val = unsafe { *window.get_unchecked(i) };
        if val < min_val {
            min_val = val;
            min_idx = i;
        }
    }

    (min_val, min_idx)
}

pub(crate) fn find_min_simd<const N: usize>(window: &[f64], current: f64) -> (f64, usize) {
    let mut global_min = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut min_idx = 0;
    let search_window = unsafe { window.get_unchecked(1..) };

    let mut best_values = Simd::<f64, N>::splat(0.0);
    let mut best_start = usize::MAX;

    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::<f64, N>::from_slice(chunk);
        let mask = values.simd_le(global_min);
        if mask.any() {
            global_min = Simd::splat(values.reduce_min());
            best_values = values;
            best_start = chunk_idx;
        }
    }

    if best_start != usize::MAX {
        let i = if N <= 4 {
            best_values.simd_eq(global_min).to_bitmask().ilog2() as usize
        } else {
            let eq_mask = best_values.simd_eq(global_min);
            let mut i = N;
            while i > 0 { i -= 1; if unsafe { eq_mask.test_unchecked(i) } { break; } }
            i
        };
        min_idx = best_start * N + 1 + i;
    }
    let mut global_min = global_min[0];
    let processed_len = (search_window.len() / N) * N;
    let remainder = unsafe { search_window.get_unchecked(processed_len..) };
    if !remainder.is_empty() {
        let (rem_min, rem_idx) = find_remainder(remainder);
        if rem_min <= global_min {
            global_min = rem_min;
            min_idx = processed_len + 1 + rem_idx;
        }
    }

    if global_min < current {
        return (global_min, min_idx);
    }
    (current, window.len())
}
