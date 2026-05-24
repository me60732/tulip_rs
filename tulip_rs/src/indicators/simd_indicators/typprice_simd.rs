#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::typprice::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::Simd;

/// Computes the Typical Price for `N` assets simultaneously using SIMD parallelism.
///
/// Returns `(high + low + close) / 3` for each lane.
///
/// # Arguments
///
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `close` - Close prices for this bar.
///
/// # Returns
///
/// Typical price values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N> {
    (high + low + close) * F64Constants::THIRD
}
