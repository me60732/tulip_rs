#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vwma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vwma::indicator_by_options;
use crate::indicators::{simd_indicators::simd_types::F64Constants, vwma::State};
use std::simd::{cmp::SimdPartialEq, *};

/// SIMD-parallel state for the Volume Weighted Moving Average (VWMA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub sum: Simd<f64, N>,
    pub vol_sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
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
    /// Converts the SIMD state into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let sum = self.sum.to_array();
        let vol_sum = self.vol_sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(sum[i], vol_sum[i]));

        states
    }
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let sum = self.sum.to_array();
        let vol_sum = self.vol_sum.to_array();

        for i in 0..N {
            states[i].sum = sum[i];
            states[i].vol_sum = vol_sum[i];
        }
    }

    /// Computes one bar of the Volume Weighted Moving Average (VWMA) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Slides the window by subtracting the oldest bar's contribution and adding the current bar's,
    /// then returns `sum / vol_sum`. Returns zero for lanes where `vol_sum` is zero.
    ///
    /// # Arguments
    ///
    /// * `close` - Close prices for this bar.
    /// * `volume` - Volume for this bar.
    /// * `prev_close` - Close prices from `period` bars ago.
    /// * `prev_volume` - Volume from `period` bars ago.
    ///
    /// # Returns
    ///
    /// VWMA values for all `N` lanes.
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

/// Computes one bar of the Volume Weighted Moving Average (VWMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Thin wrapper delegating to [`SimdState::calc_simd`].
///
/// # Arguments
///
/// * `state` - Mutable SIMD state.
/// * `close` - Close prices for this bar.
/// * `volume` - Volume for this bar.
/// * `prev_close` - Close prices from `period` bars ago.
/// * `prev_volume` - Volume from `period` bars ago.
///
/// # Returns
///
/// VWMA values for all `N` lanes.
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
