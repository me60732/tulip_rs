#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vwap::indicator_by_assets;
use crate::indicators::{
    simd_indicators::typprice_simd::calc_simd as typprice_calc_simd, vwap::IndicatorState as State,
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Volume Weighted Average Price (VWAP) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    pub pv_sum: Simd<f64, N>,
    pub vol_sum: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut pv_sum = [0.0; N];
        let mut vol_sum = [0.0; N];

        for i in 0..N {
            pv_sum[i] = states[i].pv_sum;
            vol_sum[i] = states[i].vol_sum;
        }
        Self {
            pv_sum: Simd::from_array(pv_sum),
            vol_sum: Simd::from_array(vol_sum),
        }
    }

    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let pv_sum = self.pv_sum.to_array();
        let vol_sum = self.vol_sum.to_array();

        for i in 0..N {
            states[i].pv_sum = pv_sum[i];
            states[i].vol_sum = vol_sum[i];
        }
    }
    /// Computes one bar of VWAP for `N` assets simultaneously using SIMD parallelism.
    ///
    /// Accumulates `typprice × volume` and `volume` into the running per-lane sums,
    /// then returns `(vwap, typprice)` for each lane.
    /// `typprice = (high + low + close) / 3`.
    ///
    /// # Arguments
    ///
    /// * `high`   - High prices for this bar (one per lane).
    /// * `low`    - Low prices for this bar (one per lane).
    /// * `close`  - Close prices for this bar (one per lane).
    /// * `volume` - Volume for this bar (one per lane).
    ///
    /// # Returns
    ///
    /// `(vwap, typprice)` — the cumulative VWAP and the typical price for each lane.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        volume: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let tp = typprice_calc_simd(high, low, close);
        self.pv_sum += tp * volume;
        self.vol_sum += volume;
        (self.pv_sum / self.vol_sum, tp)
    }
}
