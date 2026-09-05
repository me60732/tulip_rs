#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::smaenvelope::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::smaenvelope::indicator_by_options;


use crate::indicators::{
    simd_indicators::sma_simd::SimdState as SmaSimdState,
    smaenvelope::State
};
use crate::types::Warm;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::Simd;

pub struct SimdState<const N: usize> {
    sma_state: SmaSimdState<N>,
    percentage: Simd<f64, N>
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(sma_state: SmaSimdState<N>)],
         scalar: [percentage]
    );
    crate::simd_state_write!(
         sub: [(sma_state: SmaSimdState<N>)],
         scalar: []
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>
    ) -> Self::Outputs {
        let sma = self.sma_state.calc(inputs);
        let step = sma * self.percentage;
    
        (sma - step, sma, sma + step)
    }
}


