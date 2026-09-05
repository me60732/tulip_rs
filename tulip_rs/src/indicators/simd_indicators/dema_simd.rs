use crate::indicators::dema::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::dema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::dema::indicator_by_options;

use crate::indicators::simd_indicators::{
    ema_simd::{calc_simd as calc_ema_simd, SimdState as EmaSimdState}, 
    simd_types::F64Constants,
};
use crate::types::Warm;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{Simd, StdFloat};
use std::ops::{Deref, DerefMut};
/// SIMD-parallel state for computing the Double Exponential Moving Average (DEMA) across `N`
/// assets simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub ema_state: EmaSimdState<N>,
    pub ema2: Simd<f64, N>,
}
impl<const N: usize> Deref for SimdState<N> {
    type Target = EmaSimdState<N>;
    fn deref(&self) -> &Self::Target { &self.ema_state }
}
impl<const N: usize> DerefMut for SimdState<N> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ema_state }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(ema_state: EmaSimdState<N>)],
         scalar: [ema2]
    );
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        value: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let ema1 = self.ema_state.calc(value);
        self.ema2 = calc_ema_simd(ema1, self.ema2, self.multiplier, self.inv_multiplier);
        //(F64Constants::TWO * state.ema1 - state.ema2, state.ema1)
        (
            ema1.mul_add(F64Constants::TWO, -self.ema2),
            ema1,
        )
    }
}


