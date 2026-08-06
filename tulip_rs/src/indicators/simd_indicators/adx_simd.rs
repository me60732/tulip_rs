pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::adx::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adx::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adx::indicator_by_options;
use crate::indicators::simd_indicators::{
    dx_simd::SimdState as DxSimdState, wilders_simd::SimdState as WildersSimdState,
};
use crate::types::Warm;
use std::simd::Simd;

/// SIMD-parallel state for computing the Average Directional Index (ADX) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub dx_state: DxSimdState<N>,
    pub wilders_state: WildersSimdState<N>,
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (dx, atr, tr) = self.dx_state.calc(inputs);
        let adx = self.wilders_state.calc(dx);
        (adx, dx, atr, tr)
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;

    crate::simd_state_impl!(
        sub: [(dx_state: DxSimdState<N>), (wilders_state: WildersSimdState<N>)],
        scalar: []
    );
}
