use crate::indicators::dema::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dema::indicator_by_options;

use crate::indicators::simd_indicators::{ema_simd::calc_simd as calc_ema_simd, simd_types::F64Constants};
use std::simd::{Simd, StdFloat};

pub struct SimdState<const N: usize> {
    pub ema1: Simd<f64, N>,
    pub ema2: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new_mut_ref(states: &[&mut State]) -> Self {
        let mut ema1 = [0.0; N];
        let mut ema2 = [0.0; N];

        for i in 0..N {
            ema1[i] = states[i].ema1;
            ema2[i] = states[i].ema2;
        }
        Self {
            ema1: Simd::from_array(ema1),
            ema2: Simd::from_array(ema2),
        }
    }
    pub fn new(states: &[&State]) -> Self {
        let mut ema1 = [0.0; N];
        let mut ema2 = [0.0; N];

        for i in 0..N {
            ema1[i] = states[i].ema1;
            ema2[i] = states[i].ema2;
        }
        Self {
            ema1: Simd::from_array(ema1),
            ema2: Simd::from_array(ema2),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let ema1 = self.ema1.to_array();
        let ema2 = self.ema2.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(ema1[i], ema2[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let ema1 = self.ema1.to_array();
        let ema2 = self.ema2.to_array();

        for i in 0..N {
            states[i].ema1 = ema1[i];
            states[i].ema2 = ema2[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    multiplier: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.ema1 = calc_ema_simd(value, state.ema1, multiplier);
    state.ema2 = calc_ema_simd(state.ema1, state.ema2, multiplier);
    //(F64Constants::TWO * state.ema1 - state.ema2, state.ema1)
    (
        state.ema1.mul_add(F64Constants::TWO, -state.ema2),
        state.ema1,
    )
}
