#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wad::indicator_by_assets;
use crate::indicators::{simd_indicators::simd_types::F64Constants, wad::IndicatorState as State};
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, *};
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for the Williams Accumulation/Distribution (WAD) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub prev_close: Simd<f64, N>,
    pub wad: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
         sub: [],
         scalar: [prev_close, wad]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close): Self::Inputs<'a>
    ) -> Self::Outputs {
        // Create masks for different conditions
        let close_gt_prev = close.simd_gt(self.prev_close);
        let close_lt_prev = close.simd_lt(self.prev_close);

        // Only calculate increments where needed using masks
        // For up trend: close - min(prev_close, low)
        let up_increment =
            close_gt_prev.select(close - self.prev_close.simd_min(low), F64Constants::ZERO);

        // For down trend: close - max(prev_close, high)
        let down_increment =
            close_lt_prev.select(close - self.prev_close.simd_max(high), F64Constants::ZERO);

        // Combine the increments (only one will be non-zero per lane)
        let increment = up_increment + down_increment;

        self.wad += increment;
        self.prev_close = close;

        self.wad
    }
}
