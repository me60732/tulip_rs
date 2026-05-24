use crate::indicators::obv::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::obv::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{cmp::SimdPartialOrd, *};

/// SIMD-parallel state for the On Balance Volume (OBV) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub obv: Simd<f64, N>,
    pub prev_close: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut obv = [0.0; N];
        let mut prev_close = [0.0; N];

        for i in 0..N {
            obv[i] = states[i].obv;
            prev_close[i] = states[i].prev_close;
        }
        Self {
            obv: Simd::from_array(obv),
            prev_close: Simd::from_array(prev_close),
        }
    }
    /*pub fn to_states(&self) -> [State; N] {
        let obv = self.obv.to_array();
        let prev_close = self.prev_close.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(obv[i], prev_close[i]));

        states
    }*/
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let obv = self.obv.to_array();
        let prev_close = self.prev_close.to_array();

        for i in 0..N {
            states[i].obv = obv[i];
            states[i].prev_close = prev_close[i];
        }
    }
    /// Computes one bar of the On Balance Volume (OBV) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Adds volume when close is higher than the previous close, subtracts when lower,
    /// and keeps OBV unchanged when close equals the previous close.
    ///
    /// # Arguments
    ///
    /// * `close` - Close prices for this bar.
    /// * `volume` - Volume for this bar.
    ///
    /// # Returns
    ///
    /// Updated OBV values for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(&mut self, close: Simd<f64, N>, volume: Simd<f64, N>) -> Simd<f64, N> {
        // More careful branch-free approach
        let gt_mask = close.simd_gt(self.prev_close);
        let lt_mask = close.simd_lt(self.prev_close);

        // Only add/subtract when condition is true
        let volume_change = gt_mask.select(volume, lt_mask.select(-volume, F64Constants::ZERO));

        self.obv = self.obv + volume_change;
        self.prev_close = close;
        self.obv
    }
}
