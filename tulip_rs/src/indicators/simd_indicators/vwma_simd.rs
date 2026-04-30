#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vwma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vwma::indicator_by_options;
use crate::indicators::{simd_indicators::simd_types::F64Constants, vwma::State};
use std::simd::{cmp::SimdPartialEq, *};

pub struct SimdState<const N: usize> {
    pub sum: Simd<f64, N>,
    pub vol_sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut sum = [0.0; N];
        let mut vol_sum = [0.0; N];

        for i in 0..N {
            sum[i] = states[i].sum;
            vol_sum[i] = states[i].vol_sum;
        }
        Self {
            sum: Simd::from_array(sum),
            vol_sum: Simd::from_array(vol_sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let sum = self.sum.to_array();
        let vol_sum = self.vol_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(sum[i], vol_sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let sum = self.sum.to_array();
        let vol_sum = self.vol_sum.to_array();

        for i in 0..N {
            states[i].sum = sum[i];
            states[i].vol_sum = vol_sum[i];
        }
    }

    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        close: Simd<f64, N>,
        volume: Simd<f64, N>,
        prev_close: Simd<f64, N>,
        prev_volume: Simd<f64, N>,
    ) -> Simd<f64, N> {
        // Add new bar's contribution.
        self.sum += (close * volume) - (prev_close * prev_volume);
        self.vol_sum += volume - prev_volume;

        // Create a mask for non-zero slow_sma values
        let non_zero_mask = self.vol_sum.simd_ne(F64Constants::ZERO);
        let result = self.sum / self.vol_sum;

        non_zero_mask.select(result, F64Constants::ZERO)
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
    prev_close: Simd<f64, N>,
    prev_volume: Simd<f64, N>,
) -> Simd<f64, N> {
    state.calc_simd(close, volume, prev_close, prev_volume)
}
