pub use crate::indicators::simd_indicators::di_simd::SimdState;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dx::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dx::indicator_by_options;

use crate::indicators::simd_indicators::{
    di_simd::calc_diup_didown_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, Simd};

/// Computes one bar of the Directional Movement Index (DX) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Delegates to the DI SIMD routine for updating DM+ / DM- / ATR, then computes
/// `DX = 100 * |DI+ - DI-| / (DI+ + DI-)` for all lanes.
///
/// # Arguments
///
/// * `state` - Mutable shared SIMD state (DM+ / DM- and ATR sub-states).
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `close` - Close prices for this bar.
/// * `multiplier` - Per-lane Wilder smoothing decay factor.
///
/// # Returns
///
/// A tuple `(dx, atr, tr)` for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (_, _, atr, tr) = calc_diup_didown_simd(state, high, low, close, multipliers);

    let dx = calc_dx_simd(state);

    (dx, atr, tr)
}
/// Derives the DX value from already-updated DI+ / DI- and ATR state lanes.
#[inline(always)]
pub(crate) fn calc_dx_simd<const N: usize>(state: &mut SimdState<N>) -> Simd<f64, N> {
    let di_up = state.di_state.dmup;
    let di_down = state.di_state.dmdown;

    let dm_diff = (di_up - di_down).abs();
    let dm_sum = di_up + di_down;
    (dm_diff * F64Constants::HUNDRED / dm_sum).simd_max(F64Constants::ZERO)
}
