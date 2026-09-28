use crate::indicators::ppo::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::ppo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::ppo::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    ema_simd::SimdState as EmaSimdState, simd_types::F64Constants,
};
use crate::types::Warm;
use std::simd::{num::SimdFloat, Simd};
/// SIMD-parallel state for the Percentage Price Oscillator (PPO) indicator, holding `N` lanes of per-asset state.
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
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let short_ema = self.short_ema.calc(real);
        let long_ema = self.long_ema.calc(real);
        // Clamp guards ONLY the divisor (matches the scalar `Ppo::State::calc`
        // exactly, per lane); the subtraction reads the raw EMAs.
        (
            (short_ema - long_ema) * F64Constants::HUNDRED
                / long_ema.simd_max(F64Constants::EPSILON),
            short_ema,
            long_ema,
        )
    }
}
