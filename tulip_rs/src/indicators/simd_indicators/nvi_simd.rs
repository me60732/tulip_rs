use crate::indicators::nvi::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::nvi::indicator_by_assets;
use std::simd::{cmp::SimdPartialOrd, *};

pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for the Negative Volume Index (NVI) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    nvi: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
         sub: [],
         scalar: [nvi, close, volume]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(&mut self, (close, volume): Self::Inputs<'a>) -> Self::Outputs {
        // Create a mask for where volume < state.volume
        let mask = volume.simd_lt(self.volume);

        // Calculate the new NVI value conditionally using SIMD select
        self.nvi = mask.select(close / self.close * self.nvi, self.nvi);

        (self.close, self.volume) = (close, volume);
        self.nvi
    }
}
