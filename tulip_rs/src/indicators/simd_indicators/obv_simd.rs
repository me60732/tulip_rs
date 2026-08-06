use crate::indicators::obv::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::obv::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{cmp::SimdPartialOrd, *};
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for the On Balance Volume (OBV) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub obv: Simd<f64, N>,
    pub prev_close: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
         sub: [],
         scalar: [obv, prev_close]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    
    #[inline(always)]
    fn calc<'a>(&mut self, (close, volume): Self::Inputs<'a>) -> Self::Outputs {
        // More careful branch-free approach
        let gt_mask = close.simd_gt(self.prev_close);
        let lt_mask = close.simd_lt(self.prev_close);

        // Only add/subtract when condition is true
        let volume_change = gt_mask.select(volume, lt_mask.select(-volume, F64Constants::ZERO));

        self.obv = self.obv + volume_change;
        self.prev_close = close;
        self.obv
    }
}
