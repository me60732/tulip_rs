#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::bop::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::num::SimdFloat;
use std::simd::Simd;
//use crate::math_simd::fast_max
/// Computes the Balance of Power (BOP) for `N` assets simultaneously using SIMD parallelism.
///
/// BOP = `(close - open) / max(high - low, epsilon)` for each bar. The high-low denominator
/// is clamped to [`F64Constants::EPSILON`] to prevent division by zero on flat bars.
///
/// # Arguments
///
/// * `open` - Open prices for this bar.
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `close` - Close prices for this bar.
///
/// # Returns
///
/// BOP values in the range `[-1.0, 1.0]` for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    open: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N> {
    let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
    (close - open) / hl_diff
}
