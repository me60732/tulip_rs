#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::rsi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::rsi::indicator_by_options;

use crate::indicators::rsi::State;
use crate::indicators::simd_indicators::{cmo_simd::up_down_simd, simd_types::F64Constants};
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Relative Strength Index (RSI) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub up_sum: Simd<f64, N>,
    pub down_sum: Simd<f64, N>,
    pub prev_real: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut up_sum = [0.0; N];
        let mut down_sum = [0.0; N];
        let mut prev_real = [0.0; N];
        for i in 0..N {
            up_sum[i] = states[i].up_sum;
            down_sum[i] = states[i].down_sum;
            prev_real[i] = states[i].prev_real;
        }
        Self {
            up_sum: Simd::from_array(up_sum),
            down_sum: Simd::from_array(down_sum),
            prev_real: Simd::from_array(prev_real),
        }
    }
    /// Converts the SIMD state into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();
        let prev_real = self.prev_real.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(prev_real[i], up_sum[i], down_sum[i]));

        states
    }
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let up_sum = self.up_sum.to_array();
        let down_sum = self.down_sum.to_array();
        let prev_real = self.prev_real.to_array();

        for i in 0..N {
            states[i].up_sum = up_sum[i];
            states[i].down_sum = down_sum[i];
            states[i].prev_real = prev_real[i];
        }
    }
    /// Initialises the RSI SIMD state from raw input slices by computing the first
    /// smoothed up/down averages over `period` bars.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Per-lane input price slices; must each contain at least `period + 1` values.
    /// * `period` - Look-back period for the RSI calculation.
    ///
    /// # Returns
    ///
    /// A fully-initialised [`SimdState`] ready to be updated bar-by-bar.
    pub fn init_state<'a>(inputs: &[&'a [f64]; N], period: usize) -> SimdState<N> {
        let (mut up_sum, mut down_sum) = (Simd::splat(0.0), Simd::splat(0.0));
        let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());
        let mut val = Simd::splat(0.0);
        let period_simd = Simd::splat(period as f64);
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
            val = values;
        }
        up_sum /= period_simd;
        down_sum /= period_simd;
        SimdState {
            up_sum,
            down_sum,
            prev_real: val,
        }
    }
    /// Computes one bar of the Relative Strength Index (RSI) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Applies Wilder smoothing to the up and down sums and returns
    /// `100 * up_sum / (up_sum + down_sum)`.
    ///
    /// # Arguments
    ///
    /// * `cur_real` - Current prices for this bar.
    /// * `multiplier` - Per-lane Wilder smoothing factor `(period - 1) / period`.
    ///
    /// # Returns
    ///
    /// RSI values (0–100) for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(&mut self, cur_real: Simd<f64, N>, multiplier: Simd<f64, N>) -> Simd<f64, N> {
        let (up, down) = up_down_simd(cur_real, self.prev_real);

        //self.up_sum = (up - self.up_sum) * multiplier + self.up_sum;
        //self.down_sum = (down - self.down_sum) * multiplier + self.down_sum;
        self.up_sum = (up - self.up_sum).mul_add(multiplier, self.up_sum);
        //down_sum = (down - down_sum) * multiplier + down_sum;
        self.down_sum = (down - self.down_sum).mul_add(multiplier, self.down_sum);
        self.prev_real = cur_real;

        F64Constants::HUNDRED * (self.up_sum / (self.up_sum + self.down_sum))
    }
}
