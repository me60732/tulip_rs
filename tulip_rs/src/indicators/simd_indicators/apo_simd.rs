#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::apo::indicator_by_assets;
use crate::indicators::simd_indicators::ema_simd::SimdState as EmaSimdState;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::apo::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use std::simd::Simd;
use crate::indicators::apo::State;
use crate::types::Warm;

/// SIMD-parallel state for computing the Absolute Price Oscillator (APO) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub short_ema: EmaSimdState<N>,
    pub long_ema: EmaSimdState<N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(short_ema: EmaSimdState<N>), (long_ema: EmaSimdState<N>)],
         scalar: []
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        real: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let short_ema = self.short_ema.calc(real);
        let long_ema = self.long_ema.calc(real);
    
        short_ema - long_ema
    }
}