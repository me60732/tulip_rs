use std::simd::{num::SimdFloat, Simd};

use crate::indicators::cmo::State;
use crate::indicators::simd_indicators::simd_types::F64Constants;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cmo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cmo::indicator_by_options;
//use crate::math_simd::fast_max;
pub struct SimdState<const N: usize> {
    pub up_sum: Simd<f64, N>,
    pub down_sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut up_sum = [0.0; N];
        let mut down_sum = [0.0; N];

        for i in 0..N {
            up_sum[i] = states[i].up_sum;
            down_sum[i] = states[i].down_sum;
        }
        Self {
            up_sum: Simd::from_array(up_sum),
            down_sum: Simd::from_array(down_sum),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(up_sum[i], down_sum[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();

        for i in 0..N {
            states[i].up_sum = up_sum[i];
            states[i].down_sum = down_sum[i];
        }
    }
    pub fn init_state<'a>(inputs: &[&'a [f64]; N], period: usize) -> SimdState<N> {
        let (mut up_sum, mut down_sum) = (Simd::splat(0.0), Simd::splat(0.0));
        let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());
        //for i in 1..period+1 {
        for i in 1..period + 1 {
            let values =
                Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
            let prev_values = Simd::from_array(std::array::from_fn(|j| unsafe {
                *input_ptrs[j].add(i - 1)
            }));
            let (up, down) = up_down_simd(values, prev_values);
            up_sum += up;
            down_sum += down;
        }
        SimdState { up_sum, down_sum }
    }
}

#[inline(always)]
pub fn up_down_simd<const N: usize>(
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let diff = value - prev_value;
    (
        diff.simd_max(F64Constants::ZERO),
        (-diff).simd_max(F64Constants::ZERO),
    )
}
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    prev_real_0: Simd<f64, N>,
    prev_real_1: Simd<f64, N>,
    cur_real: Simd<f64, N>,
    prior_real: Simd<f64, N>,
) -> Simd<f64, N> {
    let (old_up, old_down) = up_down_simd(prev_real_1, prev_real_0);
    let (up, down) = up_down_simd(cur_real, prior_real);
    state.up_sum += up - old_up;
    state.down_sum += down - old_down;

    F64Constants::HUNDRED * (state.up_sum - state.down_sum) / (state.up_sum + state.down_sum)
}
