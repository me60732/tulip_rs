#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::elderray::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::elderray::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::ema_simd::SimdState as EmaSimdState;
use crate::indicators::elderray::State;
use crate::types::Warm;

use std::simd::Simd;
use std::ops::{Deref, DerefMut};
#[repr(transparent)]
pub struct SimdState<const N: usize>(pub EmaSimdState<N>);
impl<const N: usize> Deref for SimdState<N> {
    type Target = EmaSimdState<N>;
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
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close): Self::Inputs<'a>
    ) -> Self::Outputs {
        let ema = self.0.calc(close);
    
        (high - ema, low - ema, ema)
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        Self(EmaSimdState::from_states(&mut inner))
    }
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        self.0.write_states(&mut inner)
    }
}

