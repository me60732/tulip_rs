use crate::indicators::linreg::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::linreg::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::linreg::indicator_by_options;

use std::simd::{Simd, StdFloat};

pub struct SimdState<const N: usize> {
    pub sum_x: Simd<f64, N>,
    pub sum_y: Simd<f64, N>,
    pub sum_xy: Simd<f64, N>,
    pub per: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new_mut_ref(states: &[&mut State]) -> Self {
        let mut sum_x = [0.0; N];
        let mut sum_y = [0.0; N];
        let mut sum_xy = [0.0; N];
        let mut per = [0.0; N];

        for i in 0..N {
            sum_x[i] = states[i].sum_x;
            sum_y[i] = states[i].sum_y;
            sum_xy[i] = states[i].sum_xy;
            per[i] = states[i].per;
        }
        Self {
            sum_x: Simd::from_array(sum_x),
            sum_y: Simd::from_array(sum_y),
            sum_xy: Simd::from_array(sum_xy),
            per: Simd::from_array(per),
        }
    }
    pub fn new(states: &[&State]) -> Self {
        let mut sum_x = [0.0; N];
        let mut sum_y = [0.0; N];
        let mut sum_xy = [0.0; N];
        let mut per = [0.0; N];

        for i in 0..N {
            sum_x[i] = states[i].sum_x;
            sum_y[i] = states[i].sum_y;
            sum_xy[i] = states[i].sum_xy;
            per[i] = states[i].per;
        }
        Self {
            sum_x: Simd::from_array(sum_x),
            sum_y: Simd::from_array(sum_y),
            sum_xy: Simd::from_array(sum_xy),
            per: Simd::from_array(per),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let sum_x = self.sum_x.to_array();
        let sum_y = self.sum_y.to_array();
        let sum_xy = self.sum_xy.to_array();
        let per = self.per.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(sum_x[i], sum_y[i], sum_xy[i], per[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let sum_x = self.sum_x.to_array();
        let sum_y = self.sum_y.to_array();
        let sum_xy = self.sum_xy.to_array();

        for i in 0..N {
            states[i].sum_x = sum_x[i];
            states[i].sum_y = sum_y[i];
            states[i].sum_xy = sum_xy[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    prev_value: Simd<f64, N>,
    value: Simd<f64, N>,
    period: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (sum_x, mut sum_y, mut sum_xy, per) = (state.sum_x, state.sum_y, state.sum_xy, state.per);

    // FMA: (value * period) + sum_xy
    sum_xy = value.mul_add(period, sum_xy);
    sum_y += value;

    // slope = (period * sum_xy - sum_x * sum_y) * per
    let slope = sum_x.mul_add(-sum_y, period * sum_xy) * per;

    // intercept = (sum_y - slope * sum_x) / period
    let intercept = slope.mul_add(-sum_x, sum_y) / period;

    // linreg = intercept + slope * period
    let linreg = slope.mul_add(period, intercept);

    sum_xy -= sum_y;
    sum_y -= prev_value;

    (state.sum_y, state.sum_xy) = (sum_y, sum_xy);
    (linreg, slope, intercept)
}
