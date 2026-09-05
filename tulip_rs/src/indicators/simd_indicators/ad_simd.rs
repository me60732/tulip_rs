#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::ad::indicator_by_assets;
pub use crate::indicator_types::{TState, TSimdState};
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{cmp::SimdPartialOrd, Select, Simd};
use crate::indicators::ad::State;
pub struct SimdState<const N: usize> {
    pub ad: Simd<f64, N>
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, volume): Self::Inputs<'a>
    ) -> Simd<f64, N> {
        let range = high - low;
    
        // Create mask for valid ranges (>= min)
        let valid_mask = range.simd_ge(F64Constants::EPSILON);
    
        // Calculate the AD formula for all elements (may produce NaN/Inf for invalid ranges)
        let calculated_ad = self.ad + (close - low - high + close) / range * volume;
    
        // Select between original AD (for invalid range) and calculated AD (for valid range)
        self.ad = valid_mask.select(calculated_ad, self.ad);
        self.ad
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
        sub: [],
        scalar: [ad]
    );
}
