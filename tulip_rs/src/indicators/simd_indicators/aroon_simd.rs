use crate::indicators::aroon::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::aroon::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::aroon::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, min_simd::SimdState as SimdMinState,
};
use std::simd::{num::SimdUint, Simd};

/// SIMD-parallel state for computing the Aroon indicator across `N` assets simultaneously.
/// Wraps dedicated min/max ring-buffer SIMD states for tracking the lookback window.
pub struct SimdState<const N: usize> {
    /// SIMD state for the rolling minimum (tracks periods since the lowest low).
    min_state: SimdMinState<N>,
    /// SIMD state for the rolling maximum (tracks periods since the highest high).
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

    /// SIMD computation trait for the Aroon indicator, operating on `N` asset lanes simultaneously.
    pub trait Calc<const N: usize> {
        /// Computes Aroon Down and Aroon Up for one bar across `N` asset lanes using SIMD.
        ///
        /// Delegates to the underlying min/max SIMD states to find the trailing index of the
        /// period low and period high respectively, then scales both by `multiplier = 100 / period`.
        /// The `WINDOW_LANES` const parameter tunes the inner ring-buffer SIMD width.
        ///
        /// # Safety
        ///
        /// Callers must ensure that `high[lane]` and `low[lane]` point to valid memory at
        /// index `i`, and that `i >= period` so the lookback window is fully in bounds.
        ///
        /// # Returns
        ///
        /// A tuple `(aroon_down, aroon_up)` of SIMD vectors for all `N` lanes.
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            period: usize,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            period: usize,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let period_simd = Simd::splat(period);

            let (_, min_trail) = self
                .min_state
                .calc_unchecked_simd::<WINDOW_LANES>(low, i, period);
            let (_, max_trail) = self
                .max_state
                .calc_unchecked_simd::<WINDOW_LANES>(high, i, period);

            let aroon_up = (period_simd - max_trail).cast() * multiplier;
            let aroon_down = (period_simd - min_trail).cast() * multiplier;

            (aroon_down, aroon_up)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };
    /// SIMD computation trait for the Aroon indicator, operating on `N` option lanes simultaneously.
    ///
    /// Unlike the `assets` variant, `i` and `period` are SIMD vectors so each lane can be at
    /// a different bar position and use a different lookback period.
    pub trait Calc<const N: usize> {
        /// Computes Aroon Down and Aroon Up for one bar across `N` option lanes using SIMD.
        ///
        /// Each lane independently looks up the trailing-min and trailing-max indices within
        /// its own `period` window, then scales by its own `multiplier`.
        ///
        /// # Safety
        ///
        /// Callers must ensure that `high[lane]` and `low[lane]` point to valid memory at
        /// `i[lane]`, and that `i[lane] >= period[lane]` for every lane.
        ///
        /// # Returns
        ///
        /// A tuple `(aroon_down, aroon_up)` of SIMD vectors for all `N` lanes.
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            period: Simd<usize, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>);
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            period: Simd<usize, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let (_, min_trail) = self.min_state.calc_unchecked_simd(low, i, period);
            let (_, max_trail) = self.max_state.calc_unchecked_simd(high, i, period);

            let aroon_up = (period - max_trail).cast() * multiplier;
            let aroon_down = (period - min_trail).cast() * multiplier;

            (aroon_down, aroon_up)
        }
    }
}
