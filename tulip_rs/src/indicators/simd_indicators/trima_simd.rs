#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trima::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trima::indicator_by_options;

use crate::indicators::trima::State;
use std::simd::Simd;

pub struct SimdState<const N: usize> {
    pub weight_sum: Simd<f64, N>,
    pub lead_sum: Simd<f64, N>,
    pub trail_sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut weight_sum = [0.0; N];
        let mut lead_sum = [0.0; N];
        let mut trail_sum = [0.0; N];

        for i in 0..N {
            weight_sum[i] = states[i].weight_sum;
            lead_sum[i] = states[i].lead_sum;
            trail_sum[i] = states[i].trail_sum;
        }
        Self {
            weight_sum: Simd::from_array(weight_sum),
            lead_sum: Simd::from_array(lead_sum),
            trail_sum: Simd::from_array(trail_sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let weight_sum = self.weight_sum.to_array();
        let lead_sum = self.lead_sum.to_array();
        let trail_sum = self.trail_sum.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(weight_sum[i], lead_sum[i], trail_sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let weight_sum = self.weight_sum.to_array();
        let lead_sum = self.lead_sum.to_array();
        let trail_sum = self.trail_sum.to_array();

        for i in 0..N {
            states[i].weight_sum = weight_sum[i];
            states[i].lead_sum = lead_sum[i];
            states[i].trail_sum = trail_sum[i];
        }
    }
    pub fn calc_simd(
        &mut self,
        real: Simd<f64, N>,
        lsi: Simd<f64, N>,
        tsi1: Simd<f64, N>,
        tsi2: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> Simd<f64, N> {
        //calc_simd(self, real, lsi, tsi1, tsi2, multiplier)
        self.weight_sum += real;
        let trima = self.weight_sum * multiplier;
        self.lead_sum += real;
        self.weight_sum += self.lead_sum - self.trail_sum;
        self.lead_sum -= lsi;
        self.trail_sum += tsi1 - tsi2;

        trima
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    real: Simd<f64, N>,
    lsi: Simd<f64, N>,
    tsi1: Simd<f64, N>,
    tsi2: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    state.calc_simd(real, lsi, tsi1, tsi2, multiplier)
}
