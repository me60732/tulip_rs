#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::zlema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::zlema::indicator_by_options;

use crate::indicators::zlema::State;
use std::simd::{Simd, StdFloat};

pub struct SimdState<const N: usize> {
    pub zlema: Simd<f64, N>,
    pub per: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut zlema = [0.0; N];
        let mut per = [0.0; N];
        let mut multiplier = [0.0; N];

        for i in 0..N {
            zlema[i] = states[i].zlema;
            per[i] = states[i].per;
            multiplier[i] = states[i].multiplier;
        }
        Self {
            zlema: Simd::from_array(zlema),
            per: Simd::from_array(per),
            multiplier: Simd::from_array(multiplier),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let zlema = self.zlema.to_array();
        let per = self.per.to_array();
        let multiplier = self.multiplier.to_array();

        let states: [State; N] = std::array::from_fn(|i| State {
            zlema: zlema[i],
            per: per[i],
            multiplier: multiplier[i],
        });

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let zlema = self.zlema.to_array();

        for i in 0..N {
            states[i].zlema = zlema[i];
        }
    }
    #[inline(always)]
    pub fn calc_simd(&mut self, current: Simd<f64, N>, lagged: Simd<f64, N>) -> Simd<f64, N> {
        let adjusted = current + (current - lagged);
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema
    }
}

pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    current: Simd<f64, N>,
    lagged: Simd<f64, N>,
) -> Simd<f64, N> {
    state.calc_simd(current, lagged)
}
