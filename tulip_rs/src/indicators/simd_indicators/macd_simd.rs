#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::macd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::macd::indicator_by_options;
use crate::types::Warm;
use crate::indicators::macd::State;
use crate::indicators::simd_indicators::ema_simd::SimdState as EmaSimdState;
use std::simd::Simd;
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for computing the MACD indicator across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub short_ema: EmaSimdState<N>,
    pub long_ema: EmaSimdState<N>,
    pub signal_state: EmaSimdState<N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(short_ema: EmaSimdState<N>), (long_ema: EmaSimdState<N>), (signal_state: EmaSimdState<N>)],
         scalar: []
    );
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        value: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let short_ema = self.short_ema.calc(value);
        let long_ema = self.long_ema.calc(value);
    
        let macd_value = short_ema - long_ema;
        let signal = self.signal_state.calc(macd_value);
    
        (macd_value, signal, macd_value - signal, short_ema, long_ema)
    }
}


