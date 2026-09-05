#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::vosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::vosc::indicator_by_options;
use crate::indicators::simd_indicators::{
    simd_types::F64Constants, sma_simd::SimdState as SmaSimdState,
};
use crate::indicators::vosc::State;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{cmp::SimdPartialEq, *};
use crate::types::Warm;

/// SIMD-parallel state for the Volume Oscillator (VOSC) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub short_state: SmaSimdState<N>,
    pub long_state: SmaSimdState<N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(short_state: SmaSimdState<N>), (long_state: SmaSimdState<N>)],
         scalar: []
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    /// Computes one bar of the Volume Oscillator (VOSC) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Updates the short-term and long-term volume SMAs and returns
    /// `(fast_sma - slow_sma) * 100 / slow_sma`. Returns zero for lanes where `slow_sma` is zero.
    ///
    /// # Arguments
    ///
    /// * `vols` - Tuple of `(current_volume, prev_short_volume, prev_long_volume)` used by the
    ///   respective SMA windows.
    /// * `short_multiplier` - Per-lane SMA factor `1 / short_period`.
    /// * `long_multiplier` - Per-lane SMA factor `1 / long_period`.
    ///
    /// # Returns
    ///
    /// A tuple `(vosc, fast_sma, slow_sma)` for all `N` lanes.
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        vols: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let fast_sma = self.short_state.calc((vols.0, vols.1));
        let slow_sma = self.long_state.calc((vols.0, vols.2));

        // Create a mask for non-zero slow_sma values
        let non_zero_mask = slow_sma.simd_ne(F64Constants::ZERO);

        // Calculate the result for non-zero cases
        let result = (fast_sma - slow_sma) * F64Constants::HUNDRED / slow_sma;

        // Use select to return 0.0 where slow_sma is zero, otherwise return the calculated result
        (
            non_zero_mask.select(result, F64Constants::ZERO),
            fast_sma,
            slow_sma,
        )
    }
}

