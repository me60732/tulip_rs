#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vosc::indicator_by_options;
use crate::indicators::simd_indicators::{
    simd_types::F64Constants, sma_simd::calc_simd as sma_calc_simd,
};
use crate::indicators::vosc::State;

use std::simd::{cmp::SimdPartialEq, *};
pub struct SimdState<const N: usize> {
    pub short_sum: Simd<f64, N>,
    pub long_sum: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut short_sum = [0.0; N];
        let mut long_sum = [0.0; N];

        for i in 0..N {
            short_sum[i] = states[i].short_sum;
            long_sum[i] = states[i].long_sum;
        }
        Self {
            short_sum: Simd::from_array(short_sum),
            long_sum: Simd::from_array(long_sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let short_sum = self.short_sum.to_array();
        let long_sum = self.long_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(short_sum[i], long_sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let short_sum = self.short_sum.to_array();
        let long_sum = self.long_sum.to_array();

        for i in 0..N {
            states[i].short_sum = short_sum[i];
            states[i].long_sum = long_sum[i];
        }
    }

    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        vols: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        short_multiplier: Simd<f64, N>,
        long_multiplier: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let fast_sma = sma_calc_simd(&mut self.short_sum, vols.0, vols.1, short_multiplier);
        let slow_sma = sma_calc_simd(&mut self.long_sum, vols.0, vols.2, long_multiplier);

        // Create a mask for non-zero slow_sma values
        let non_zero_mask = slow_sma.simd_ne(F64Constants::ZERO);

        // Calculate the result for non-zero cases
        let result = (fast_sma - slow_sma) * F64Constants::HUNDRED / slow_sma;

        // Use select to return 0.0 where slow_sma is zero, otherwise return the calculated result
        (
            non_zero_mask.select(result, F64Constants::ZERO),
            fast_sma,
            slow_sma,
        )
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    vols: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    short_multiplier: Simd<f64, N>,
    long_multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    state.calc_simd(vols, short_multiplier, long_multiplier)
}
