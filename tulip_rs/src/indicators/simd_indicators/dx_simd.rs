pub(crate) use crate::indicators::simd_indicators::di_simd::{
    SimdState as DiSimdState, TSimdState, TState,
};

#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::dx::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::dx::indicator_by_options;

use crate::indicators::dx::State;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::ops::{Deref, DerefMut};
use std::simd::{num::SimdFloat, Simd};
use crate::types::Warm;
#[repr(transparent)]
pub struct SimdState<const N: usize>(pub DiSimdState<N>);
impl<const N: usize> Deref for SimdState<N> {
    type Target = DiSimdState<N>;
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
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (_, _, atr, tr) = self.calc_diup_didown(inputs);

        let dx = self.calc_dx();

        (dx, atr, tr)
    }
}
impl<const N: usize> SimdState<N> {
    #[inline(always)]
    pub(crate) fn calc_dx(&mut self) -> Simd<f64, N> {
        let di_up = self.di_state.dmup;
        let di_down = self.di_state.dmdown;

        let dm_diff = (di_up - di_down).abs();
        let dm_sum = di_up + di_down;
        (dm_diff * F64Constants::HUNDRED / dm_sum).simd_max(F64Constants::ZERO)
    }
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        Self(DiSimdState::from_states(&mut inner))
    }
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
        self.0.write_states(&mut inner)
    }
}
