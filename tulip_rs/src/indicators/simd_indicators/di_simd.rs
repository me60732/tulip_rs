pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::di::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::di::indicator_by_options;
use crate::indicators::{
    di::State,
    simd_indicators::{
        atr_simd::SimdState as AtrSimdState, dm_simd::SimdState as DmSimdState,
        simd_types::F64Constants,
    },
};
use crate::types::Warm;
use std::simd::{cmp::SimdPartialEq, Select, Simd};

/// SIMD-parallel state for computing the Directional Indicator (DI) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    /// Underlying Directional Movement (DM) SIMD state tracking Wilder-smoothed +DM and -DM.
    pub di_state: DmSimdState<N>,
    /// Underlying Average True Range (ATR) SIMD state used to normalise the directional movement.
    pub atr_state: AtrSimdState<N>,
}
impl<const N: usize> SimdState<N> {
    #[inline(always)]
    pub fn calc_diup_didown(
        &mut self,
        (high, low, close): (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (dmup, dmdown) = self.di_state.calc((high, low));
        let (atr, tr) = self.atr_state.partial_calc((high, low, close));
        (dmup, dmdown, atr, tr)
    }
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (dmup, dmdown, atr, tr) = self.calc_diup_didown(inputs);
        let atr_inv = F64Constants::HUNDRED / atr;
        let mut pdi = dmup * atr_inv; // multiplication
        let mut mdi = dmdown * atr_inv;

        // SIMD NaN detection and replacement
        pdi = pdi.simd_ne(pdi).select(F64Constants::ZERO, pdi); // if NaN, use 0, else use pdi

        mdi = mdi.simd_ne(mdi).select(F64Constants::ZERO, mdi); // if NaN, use 0, else use mdi

        (pdi, mdi, atr, tr)
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
        sub: [(di_state: DmSimdState<N>), (atr_state: AtrSimdState<N>)],
        scalar: []
    );
}
