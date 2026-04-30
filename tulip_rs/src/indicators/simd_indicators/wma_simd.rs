#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::wma::indicator_by_options;

use crate::indicators::{
    simd_indicators::{simd_types::F64Constants, sma_simd::calc_simd as calc_sma_simd},
    wma::State,
};
use std::simd::Simd;

pub struct SimdState<const N: usize> {
    sum: Simd<f64, N>,
    weighted_sum: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut sum = [0.0; N];
        let mut weighted_sum = [0.0; N];

        for i in 0..N {
            sum[i] = states[i].sum;
            weighted_sum[i] = states[i].weighted_sum;
        }
        Self {
            sum: Simd::from_array(sum),
            weighted_sum: Simd::from_array(weighted_sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let sum = self.sum.to_array();
        let weighted_sum = self.weighted_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(sum[i], weighted_sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let sum = self.sum.to_array();
        let sum_sq = self.weighted_sum.to_array();

        for i in 0..N {
            states[i].sum = sum[i];
            states[i].weighted_sum = sum_sq[i];
        }
    }
    pub fn init_state<'a>(inputs: &[&'a [f64]; N], period: usize) -> SimdState<N> {
        let mut sums = Simd::splat(0.0);
        let mut weighted_sum = Simd::splat(0.0);
        // Optimization: Pre-compute input pointers for the initialization loop
        let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());

        for i in 0..period {
            let values =
                Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
            sums += values;
            weighted_sum += values * (Simd::splat(i as f64) + F64Constants::ONE);
        }
        SimdState::<N> {
            sum: sums,
            weighted_sum: weighted_sum,
        }
    }
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        prev_value: Simd<f64, N>,
        value: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let (multiplier, weights, n) = multipliers;

        self.weighted_sum -= self.sum;

        let sma = calc_sma_simd(&mut self.sum, value, prev_value, multiplier);

        self.weighted_sum += value * n;

        let wma = self.weighted_sum / weights;

        (wma, sma)
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    prev_value: Simd<f64, N>,
    value: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.calc_simd(prev_value, value, multipliers)
}
