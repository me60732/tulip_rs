use std::simd::{num::SimdFloat, Simd};

use crate::indicators::cmo::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cmo::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cmo::indicator_by_options;
//use crate::math_simd::fast_max;
/// SIMD-parallel state for computing the Chande Momentum Oscillator (CMO) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Running sum of upward price changes within the lookback window.
    pub up_sum: Simd<f64, N>,
    /// Running sum of downward price changes within the lookback window.
    pub down_sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
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
    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(up_sum[i], down_sum[i]));

        states
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to [`to_states`].
    pub fn write_states(&self, states: &mut [&mut State]) {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();

        for i in 0..N {
            states[i].up_sum = up_sum[i];
            states[i].down_sum = down_sum[i];
        }
    }
    /// Initialises the CMO state by pre-computing the first window of up/down sums.
    ///
    /// Iterates over `inputs[i][1..=period]` for each of the `N` lanes, accumulating
    /// the up and down price changes using [`up_down_simd`].
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

/// Splits a price change into its up and down components across all `N` lanes.
///
/// `up = max(value - prev_value, 0)` and `down = max(prev_value - value, 0)`.
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
/// Advances the CMO by one bar for `N` assets simultaneously.
///
/// Slides the rolling window: subtracts the oldest up/down pair (`prev_real_1 - prev_real_0`)
/// and adds the new pair (`cur_real - prior_real`). Returns
/// `100 * (up_sum - down_sum) / (up_sum + down_sum)`.
///
/// # Arguments
///
/// * `state` - Mutable SIMD state with running up/down sums.
/// * `prev_real_0` - The oldest value leaving the window (two bars behind the oldest).
/// * `prev_real_1` - The oldest value leaving the window.
/// * `cur_real` - The newest close price.
/// * `prior_real` - The close price one bar before `cur_real`.
///
/// # Returns
///
/// CMO values for all `N` lanes.
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
