pub use crate::indicators::simd_indicators::linreg_simd::SimdState as LinregSimdState;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tsf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::tsf::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};
use std::ops::{Deref, DerefMut};
use crate::indicators::tsf::State;
use crate::types::Warm;
#[repr(transparent)]
pub struct SimdState<const N: usize>(pub LinregSimdState<N>);
impl<const N: usize> Deref for SimdState<N> {
    type Target = LinregSimdState<N>;
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
        Self(LinregSimdState::from_states(&mut inner))
    }
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        self.0.write_states(&mut inner)
    }
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let (linreg, slope, intercept);
        (linreg, slope, intercept) = self.0.calc(inputs);
        //let tsf = intercept + slope * (period + F64Constants::ONE);
        let tsf = slope.mul_add(self.n + F64Constants::ONE, intercept);
        (tsf, linreg, slope, intercept)
    }
}