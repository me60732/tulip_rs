#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::avgprice::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::Simd;

/// Computes the Average Price for `N` assets simultaneously using SIMD parallelism.
///
/// Average Price = `(open + high + low + close) / 4` for every bar.
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
/// Average price values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    open: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N> {
    (open + high + low + close) * F64Constants::QUATER
}
