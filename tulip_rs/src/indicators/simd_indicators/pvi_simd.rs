use crate::indicators::pvi::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::pvi::indicator_by_assets;
use std::simd::{cmp::SimdPartialOrd, *};

/// SIMD-parallel state for the Positive Volume Index (PVI) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pvi: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut pvi = [0.0; N];
        let mut close = [0.0; N];
        let mut volume = [0.0; N];

        for i in 0..N {
            pvi[i] = states[i].pvi;
            close[i] = states[i].close;
            volume[i] = states[i].volume;
        }
        Self {
            pvi: Simd::from_array(pvi),
            close: Simd::from_array(close),
            volume: Simd::from_array(volume),
        }
    }
    /// Converts the SIMD state into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let pvi = self.pvi.to_array();
        let close = self.close.to_array();
        let volume = self.volume.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(pvi[i], close[i], volume[i]));

        states
    }
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let pvi = self.pvi.to_array();
        let close = self.close.to_array();
        let volume = self.volume.to_array();

        for i in 0..N {
            states[i].pvi = pvi[i];
            states[i].close = close[i];
            states[i].volume = volume[i];
        }
    }
    /// Computes one bar of the Positive Volume Index (PVI) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Updates PVI only when volume increases relative to the previous bar:
    /// `pvi *= close / prev_close`.
    ///
    /// # Arguments
    ///
    /// * `close` - Close prices for this bar.
    /// * `volume` - Volume for this bar.
    ///
    /// # Returns
    ///
    /// Updated PVI values for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(&mut self, close: Simd<f64, N>, volume: Simd<f64, N>) -> Simd<f64, N> {
        // Create a mask for where volume < state.volume
        let mask = volume.simd_gt(self.volume);

        // Calculate the new pvi value conditionally using SIMD select
        self.pvi = mask.select(close / self.close * self.pvi, self.pvi);

        (self.close, self.volume) = (close, volume);
        self.pvi
    }
}
/// Computes one bar of the Positive Volume Index (PVI) for `N` assets simultaneously
/// using SIMD parallelism, operating on scalar-like state arguments rather than a [`SimdState`].
///
/// Updates `pvi` only when volume has increased: `pvi *= close / prev_close`.
///
/// # Arguments
///
/// * `close` - Close prices for this bar.
/// * `prev_close` - Previous bar's close prices.
/// * `volume` - Volume for this bar.
/// * `prev_volume` - Previous bar's volume.
/// * `pvi` - Current PVI values for each lane.
///
/// # Returns
///
/// Updated PVI values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    close: Simd<f64, N>,
    prev_close: Simd<f64, N>,
    volume: Simd<f64, N>,
    prev_volume: Simd<f64, N>,
    mut pvi: Simd<f64, N>,
) -> Simd<f64, N> {
    // Create a mask for where volume < state.volume
    let mask = volume.simd_gt(prev_volume);

    // Calculate the new pvi value conditionally using SIMD select
    pvi = mask.select(close / prev_close * pvi, pvi);

    pvi
}
