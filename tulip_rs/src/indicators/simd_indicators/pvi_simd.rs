use crate::indicators::pvi::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::pvi::indicator_by_assets;
use std::simd::{cmp::SimdPartialOrd, *};
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for the Positive Volume Index (PVI) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pvi: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    
    crate::simd_state_impl!(
         sub: [],
         scalar: [pvi, close, volume]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    
    #[inline(always)]
    fn calc<'a>(&mut self, (close, volume): Self::Inputs<'a>) -> Self::Outputs {
        // Create a mask for where volume < state.volume
        let mask = volume.simd_gt(self.volume);

        // Calculate the new pvi value conditionally using SIMD select
        self.pvi = mask.select(close / self.close * self.pvi, self.pvi);

        (self.close, self.volume) = (close, volume);
        self.pvi
    }
}

