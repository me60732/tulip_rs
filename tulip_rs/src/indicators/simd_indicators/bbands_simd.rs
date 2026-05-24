#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::bbands::indicator_by_assets;
/// Re-uses [`stddev_simd::SimdState`] as the state for Bollinger Bands since the rolling
/// standard deviation and SMA are the core calculations needed.
pub use crate::indicators::simd_indicators::stddev_simd::SimdState;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::bbands::indicator_by_options;

use std::simd::{Simd, StdFloat};

/// Computes one bar of Bollinger Bands for `N` assets simultaneously using SIMD parallelism.
///
/// Delegates standard-deviation and SMA computation to the embedded [`SimdState`], then
/// constructs the upper and lower bands as `middle_band ± std_dev * sd`.
///
/// # Arguments
///
/// * `state` - Mutable SIMD stddev/SMA state.
/// * `std_dev` - Band-width multiplier (same for all lanes in the by-asset path).
/// * `value` - Current bar's price values.
/// * `prev_value` - Price that is leaving the rolling window.
/// * `multiplier` - SMA reciprocal multiplier `(1 / period)`.
///
/// # Returns
///
/// A tuple `(lower_band, middle_band, upper_band)` of SIMD vectors for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    std_dev: Simd<f64, N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (sd, sma);
    (sd, sma) = state.calc_simd(value, prev_value, multiplier);

    //let upper_band = sma + std_dev * sd;
    let upper_band = std_dev.mul_add(sd, sma);
    //let lower_band = sma - std_dev * sd;
    let lower_band = (-std_dev).mul_add(sd, sma);
    (lower_band, sma, upper_band)
}
