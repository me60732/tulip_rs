#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::tema::indicator_by_options;

use crate::indicators::simd_indicators::{
    dema_simd::SimdState as DemaSimdState,
    ema_simd::calc_simd as calc_ema_simd,
    simd_types::F64Constants,
};
pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::tema::State;
use std::simd::{Simd, StdFloat};
use std::ops::{Deref, DerefMut};
use crate::types::Warm;
/// SIMD-parallel state for computing the Triple Exponential Moving Average (TEMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub dema_state: DemaSimdState<N>,
    pub ema3: Simd<f64, N>,
}
impl<const N: usize> Deref for SimdState<N> {
    type Target = DemaSimdState<N>;
    fn deref(&self) -> &Self::Target { &self.dema_state }
}
impl<const N: usize> DerefMut for SimdState<N> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.dema_state }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(dema_state: DemaSimdState<N>)],
         scalar: [ema3]
    );
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        value: Self::Inputs<'a>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (dema, ema) = self.dema_state.calc(value);
        self.ema3 = calc_ema_simd(self.dema_state.ema2, self.ema3, self.multiplier, self.inv_multiplier);
    
        (
            //F64Constants::THREE * dema_state.ema1 - F64Constants::THREE * dema_state.ema2 + state.ema3,
            ema.mul_add(
                F64Constants::THREE,
                self.dema_state.ema2.mul_add(-F64Constants::THREE, self.ema3),
            ),
            dema,
            ema,
        )
    }
}