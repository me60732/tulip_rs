#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::atr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::atr::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::{
    atr::State,
    simd_indicators::{
        tr_simd::SimdState as TrSimdState, wilders_simd::SimdState as WildersSimdState,
    },
};
use std::simd::Simd;
use crate::types::Warm;
/// SIMD-parallel state for computing the Average True Range (ATR) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub tr_state: TrSimdState<N>,
    pub wilders_state: WildersSimdState<N>,
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> (Simd<f64, N>, Simd<f64, N>) {
        let tr = self.tr_state.calc(inputs);
        let atr = self.wilders_state.calc(tr);
        (atr, tr)
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;

    crate::simd_state_impl!(
        sub: [(wilders_state: WildersSimdState<N>), (tr_state: TrSimdState<N>)],
        scalar: []
    );
}
impl<const N: usize> SimdState<N> {
    /// Advances the ATR by one bar using Wilder smoothing for all `N` lanes.
    ///
    /// Computes the True Range from `high`, `low`, and the stored `prev_close`, then blends
    /// it into the running ATR with the Wilder multiplier. Updates `prev_close`.
    ///
    /// # Returns
    ///
    /// A tuple `(atr, tr)` of SIMD vectors for all `N` lanes.

    /// Advances the ATR by one bar using the partial Wilder update for all `N` lanes.
    ///
    /// Uses the partial (non-corrected) Wilder formula, suitable for the warm-up phase
    /// before the ATR is fully initialised. Updates `prev_close`.
    ///
    /// # Returns
    ///
    /// A tuple `(atr, tr)` of SIMD vectors for all `N` lanes.
    #[inline(always)]
    pub fn partial_calc(
        &mut self,
        inputs: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let tr = self.tr_state.calc(inputs);
        let atr = self.wilders_state.partial_calc_simd(tr);
        (atr, tr)
    }
}
