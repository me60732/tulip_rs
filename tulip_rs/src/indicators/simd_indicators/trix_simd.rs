use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::trix::indicator_by_assets;

pub use crate::indicators::{
    simd_indicators::tema_simd::SimdState as TemaSimdState,
    trix::State,
};
pub use crate::indicator_types::{TSimdState, TState};

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::trix::indicator_by_options;

use std::ops::{Deref, DerefMut};
use crate::types::Warm;

#[repr(transparent)]
pub struct SimdState<const N: usize>(pub TemaSimdState<N>);
impl<const N: usize> Deref for SimdState<N> {
    type Target = TemaSimdState<N>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<const N: usize> DerefMut for SimdState<N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        Self(TemaSimdState::from_states(&mut inner))
    }
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        self.0.write_states(&mut inner)
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        value: Self::Inputs<'a>,
    ) -> Self::Outputs{
        let prev_ema3 = self.ema3;
        let (tema, dema, ema) = self.0.calc(value);
        // Compute TRIX as percentage change if previous TEMA is non-zero.
        let trix = F64Constants::HUNDRED * (self.ema3 - prev_ema3) / self.ema3;
        (trix, tema, dema, ema)
    }
}
