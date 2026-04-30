use crate::indicators::kama::State;
use crate::indicators::simd_indicators::simd_types::F64Constants;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::kama::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::kama::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd, StdFloat};

pub struct SimdState<const N: usize> {
    pub kama: Simd<f64, N>,
    pub sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut kama = [0.0; N];
        let mut sum = [0.0; N];

        for i in 0..N {
            kama[i] = states[i].kama;
            sum[i] = states[i].sum;
        }

        Self {
            kama: Simd::from_array(kama),
            sum: Simd::from_array(sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let kama = self.kama.to_array();
        let sum = self.sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(kama[i], sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let kama = self.kama.to_array();
        let sum = self.sum.to_array();

        for (i, state) in states.iter_mut().enumerate() {
            state.kama = kama[i];
            state.sum = sum[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> Simd<f64, N> {
    let (value, prev_value, last_value, old_value) = values;
    let (fast_ema, slow_ema) = multipliers;
    let (mut kama, mut sum) = (state.kama, state.sum);
    let mask = sum.simd_ne(F64Constants::ZERO);
    sum += (value - prev_value).abs() - (last_value - old_value).abs();

    let efficiency_ratio = mask.select(
        (value - last_value).abs() / sum, // When sum != 0.0
        F64Constants::ONE,                // When sum == 0.0, use 1.0
    );

    let smoothing_constant = {
        let temp = (fast_ema - slow_ema).mul_add(efficiency_ratio, slow_ema);
        temp * temp // Square it by multiplying by itself
    };

    // Optimized calculation using C-style EMA pattern
    let per1 = F64Constants::ONE - smoothing_constant;
    //kama = kama * per1 + value * smoothing_constant;
    kama = kama.mul_add(per1, value * smoothing_constant);
    (state.kama, state.sum) = (kama, sum);
    kama
}
