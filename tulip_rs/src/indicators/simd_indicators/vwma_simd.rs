#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vwma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vwma::indicator_by_options;
use crate::indicators::{simd_indicators::simd_types::F64Constants, vwma::State};
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{cmp::SimdPartialEq, *};
use crate::types::Warm;
/// SIMD-parallel state for the Volume Weighted Moving Average (VWMA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub sum: Simd<f64, N>,
    pub vol_sum: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [],
         scalar: [sum, vol_sum]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (close, volume, prev_close, prev_volume): Self::Inputs<'a>
    ) -> Simd<f64, N> {
        // Add new bar's contribution.
        self.sum += (close * volume) - (prev_close * prev_volume);
        self.vol_sum += volume - prev_volume;

        // Create a mask for non-zero slow_sma values
        let non_zero_mask = self.vol_sum.simd_ne(F64Constants::ZERO);
        let result = self.sum / self.vol_sum;

        non_zero_mask.select(result, F64Constants::ZERO)
    }
}

