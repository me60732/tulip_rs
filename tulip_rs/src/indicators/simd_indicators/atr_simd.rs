use crate::indicators::atr::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::atr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::atr::indicator_by_options;

use crate::indicators::simd_indicators::{
    tr_simd::calc_simd as calc_tr_simd,
    wilders_simd::{
        calc_simd as calc_wilders_simd, partial_calc_simd as partial_calc_wilders_simd,
    },
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Average True Range (ATR) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Current Wilder-smoothed ATR value for each asset.
    pub atr: Simd<f64, N>,
    /// Previous bar's close price for each asset, used to compute the True Range.
    pub prev_close: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut atr = [0.0; N];
        let mut prev_close = [0.0; N];

        for i in 0..N {
            atr[i] = states[i].atr;
            prev_close[i] = states[i].prev_close;
        }
        Self {
            atr: Simd::from_array(atr),
            prev_close: Simd::from_array(prev_close),
        }
    }
    /*pub fn to_states(&self) -> [State; N] {
        let atr = self.atr.to_array();
        let prev_close = self.prev_close.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(atr[i], prev_close[i]));

        states
    }*/
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let atr = self.atr.to_array();
        let prev_close = self.prev_close.to_array();

        for i in 0..N {
            states[i].atr = atr[i];
            states[i].prev_close = prev_close[i];
        }
    }
    /// Advances the ATR by one bar using Wilder smoothing for all `N` lanes.
    ///
    /// Computes the True Range from `high`, `low`, and the stored `prev_close`, then blends
    /// it into the running ATR with the Wilder multiplier. Updates `prev_close`.
    ///
    /// # Returns
    ///
    /// A tuple `(atr, tr)` of SIMD vectors for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let tr = calc_tr_simd(high, low, self.prev_close);
        self.atr = calc_wilders_simd(self.atr, tr, multipliers);
        self.prev_close = close;
        (self.atr, tr)
    }
    /// Advances the ATR by one bar using the partial Wilder update for all `N` lanes.
    ///
    /// Uses the partial (non-corrected) Wilder formula, suitable for the warm-up phase
    /// before the ATR is fully initialised. Updates `prev_close`.
    ///
    /// # Returns
    ///
    /// A tuple `(atr, tr)` of SIMD vectors for all `N` lanes.
    #[inline(always)]
    pub fn partial_calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let tr = calc_tr_simd(high, low, self.prev_close);
        self.atr = partial_calc_wilders_simd(self.atr, tr, multiplier.0);
        self.prev_close = close;
        (self.atr, tr)
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.calc_simd(high, low, close, multipliers)
}
#[inline(always)]
pub fn partial_calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.partial_calc_simd(high, low, close, multipliers)
}
