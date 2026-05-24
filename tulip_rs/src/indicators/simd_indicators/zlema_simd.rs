#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::zlema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::zlema::indicator_by_options;

use crate::indicators::zlema::State;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Zero-Lag Exponential Moving Average (ZLEMA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub zlema: Simd<f64, N>,
    pub per: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut zlema = [0.0; N];
        let mut per = [0.0; N];
        let mut multiplier = [0.0; N];

        for i in 0..N {
            zlema[i] = states[i].zlema;
            per[i] = states[i].per;
            multiplier[i] = states[i].multiplier;
        }
        Self {
            zlema: Simd::from_array(zlema),
            per: Simd::from_array(per),
            multiplier: Simd::from_array(multiplier),
        }
    }
    /// Converts the SIMD state into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let zlema = self.zlema.to_array();
        let per = self.per.to_array();
        let multiplier = self.multiplier.to_array();

        let states: [State; N] = std::array::from_fn(|i| State {
            zlema: zlema[i],
            per: per[i],
            multiplier: multiplier[i],
        });

        states
    }
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let zlema = self.zlema.to_array();

        for i in 0..N {
            states[i].zlema = zlema[i];
        }
    }
    /// Computes one bar of the Zero-Lag EMA (ZLEMA) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Adjusts the current price by `current + (current - lagged)` to remove lag,
    /// then applies EMA smoothing: `zlema = zlema * per + adjusted * multiplier`.
    ///
    /// # Arguments
    ///
    /// * `current` - Current prices for this bar.
    /// * `lagged` - Prices from `(period - 1) / 2` bars ago.
    ///
    /// # Returns
    ///
    /// Updated ZLEMA values for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(&mut self, current: Simd<f64, N>, lagged: Simd<f64, N>) -> Simd<f64, N> {
        let adjusted = current + (current - lagged);
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema
    }
}

/// Computes one bar of the Zero-Lag EMA (ZLEMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Thin wrapper delegating to [`SimdState::calc_simd`].
///
/// # Arguments
///
/// * `state` - Mutable SIMD state holding current ZLEMA values and smoothing coefficients.
/// * `current` - Current prices for this bar.
/// * `lagged` - Prices from `(period - 1) / 2` bars ago.
///
/// # Returns
///
/// Updated ZLEMA values for all `N` lanes.
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    current: Simd<f64, N>,
    lagged: Simd<f64, N>,
) -> Simd<f64, N> {
    state.calc_simd(current, lagged)
}
