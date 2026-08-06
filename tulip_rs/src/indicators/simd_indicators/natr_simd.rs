#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::natr::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::natr::indicator_by_options;
pub use crate::indicators::{
    natr::State,
    simd_indicators::atr_simd::{SimdState as AtrSimdState, TSimdState, TState},
};
use std::ops::{Deref, DerefMut};
use std::simd::Simd;
use crate::types::Warm;
#[repr(transparent)]
pub struct SimdState<const N: usize>(pub AtrSimdState<N>);
impl<const N: usize> Deref for SimdState<N> {
    type Target = AtrSimdState<N>;
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
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let (atr, tr) = self.0.calc((high, low, close));
        ((atr / close) * F64Constants::HUNDRED, atr, tr)
    }
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        Self(AtrSimdState::from_states(&mut inner))
    }
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        self.0.write_states(&mut inner)
    }
}
