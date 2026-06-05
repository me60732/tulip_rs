use crate::indicators::donchianchannel::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::donchianchannel::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::donchianchannel::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, medprice_simd::calc_simd as calc_medprice_simd,
    min_simd::SimdState as SimdMinState,
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Donchian Channel indicator across `N` assets or option-sets simultaneously.
/// Wraps dedicated min/max ring-buffer SIMD states, one per lane.
pub struct SimdState<const N: usize> {
    min_state: SimdMinState<N>,
    max_state: SimdMaxState<N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut min_state = Vec::with_capacity(N);
        let mut max_state = Vec::with_capacity(N);

        for state in states.iter_mut() {
            min_state.push(&mut state.min_state);
            max_state.push(&mut state.max_state);
        }
        let min_state = SimdMinState::new(&min_state);
        let max_state = SimdMaxState::new(&max_state);

        Self {
            min_state,
            max_state,
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut max_refs = Vec::with_capacity(N);
        let mut min_refs = Vec::with_capacity(N);

        for state in states.iter_mut() {
            max_refs.push(&mut state.max_state);
            min_refs.push(&mut state.min_state);
        }
        self.max_state.write_states(&mut max_refs);
        self.min_state.write_states(&mut min_refs);
    }
}
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::Calc as CalcMax, min_simd::assets::Calc as CalcMin,
    };

    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            look_back: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        /// Advances the Donchian Channel by one bar across `N` asset lanes simultaneously.
        ///
        /// Updates the rolling min/max windows, then computes the lower, middle, and upper
        /// channel values for all lanes in parallel.
        ///
        /// # Safety
        ///
        /// `high_ptrs[k]` and `low_ptrs[k]` must point to arrays of at least `i + 1` elements.
        ///
        /// # Arguments
        ///
        /// * `high_ptrs` / `low_ptrs` - Per-asset pointers into the high/low price arrays.
        /// * `i` - Current bar index (shared across all asset lanes).
        /// * `look_back` - Min/max sliding-window size (`= period - 1`), shared across lanes.
        ///
        /// # Returns
        ///
        /// `(lower, middle, upper)` as SIMD vectors, one value per asset lane.
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high_ptrs: [*const f64; N],
            low_ptrs: [*const f64; N],
            i: usize,
            look_back: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (min, _) = self
                .min_state
                .calc_unchecked_simd::<WINDOW_LANES>(low_ptrs, i, look_back);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd::<WINDOW_LANES>(high_ptrs, i, look_back);

            let middle = calc_medprice_simd(max, min);

            (min, middle, max)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        /// Advances the Donchian Channel by one output bar across `N` option-set lanes simultaneously.
        ///
        /// Each lane may have a different period (encoded in `look_back`), so bar indices and
        /// window sizes are SIMD vectors rather than scalars.
        ///
        /// # Safety
        ///
        /// For each lane `k`, `high_ptrs[k]` and `low_ptrs[k]` must point to arrays of at
        /// least `i[k] + 1` elements.
        ///
        /// # Arguments
        ///
        /// * `high_ptrs` / `low_ptrs` - Per-lane pointers into the high/low price arrays.
        /// * `i` - Current bar indices, one per lane.
        /// * `look_back` - Per-lane min/max window sizes (`= period - 1` for each lane).
        ///
        /// # Returns
        ///
        /// `(lower, middle, upper)` as SIMD vectors, one value per option lane.
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            high_ptrs: [*const f64; N],
            low_ptrs: [*const f64; N],
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (min, _) = self.min_state.calc_unchecked_simd(low_ptrs, i, look_back);
            let (max, _) = self.max_state.calc_unchecked_simd(high_ptrs, i, look_back);

            let middle = calc_medprice_simd(max, min);

            (min, middle, max)
        }
    }
}
