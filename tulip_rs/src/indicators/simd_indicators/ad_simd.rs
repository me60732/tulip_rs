#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ad::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{cmp::SimdPartialOrd, Select, Simd};

/// Computes one bar of the Accumulation/Distribution (AD) line for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Advances the running AD total by `(close - low - (high - close)) / (high - low) * volume`.
/// When the high-low range is below [`F64Constants::EPSILON`] the previous AD value is kept
/// unchanged for that lane to avoid a division by zero.
///
/// # Arguments
///
/// * `ad` - Current running AD value for each asset lane.
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `close` - Close prices for this bar.
/// * `volume` - Volume for this bar.
///
/// # Returns
///
/// Updated AD line values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    ad: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
) -> Simd<f64, N> {
    let range = high - low;

    // Create mask for valid ranges (>= min)
    let valid_mask = range.simd_ge(F64Constants::EPSILON);

    // Calculate the AD formula for all elements (may produce NaN/Inf for invalid ranges)
    let calculated_ad = ad + (close - low - high + close) / range * volume;

    // Select between original AD (for invalid range) and calculated AD (for valid range)
    valid_mask.select(calculated_ad, ad)
}
