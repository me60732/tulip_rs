pub use crate::indicators::simd_indicators::di_simd::SimdState;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dx::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dx::indicator_by_options;

use crate::indicators::simd_indicators::{
    di_simd::calc_diup_didown_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, Simd};

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (_, _, atr, tr) = calc_diup_didown_simd(state, high, low, close, multiplier);

    let dx = calc_dx_simd(state);

    (dx, atr, tr)
}
#[inline(always)]
pub(crate) fn calc_dx_simd<const N: usize>(state: &mut SimdState<N>) -> Simd<f64, N> {
    let di_up = state.di_state.dmup / state.atr_state.atr;
    let di_down = state.di_state.dmdown / state.atr_state.atr;

    let dm_diff = (di_up - di_down).abs();
    let dm_sum = di_up + di_down;
    (dm_diff * F64Constants::HUNDRED / dm_sum).simd_max(F64Constants::ZERO)
}
