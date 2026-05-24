#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::mom::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::mom::indicator_by_options;
use std::simd::Simd;

/// Computes one bar of the Momentum (MOM) indicator for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Returns `real - prev_real` (the raw price change over `period` bars).
///
/// # Arguments
///
/// * `real` - Current prices for this bar.
/// * `prev_real` - Prices from `period` bars ago.
///
/// # Returns
///
/// Momentum values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(real: Simd<f64, N>, prev_real: Simd<f64, N>) -> Simd<f64, N> {
    real - prev_real
}
