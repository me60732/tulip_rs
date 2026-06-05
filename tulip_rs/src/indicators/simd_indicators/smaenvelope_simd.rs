#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::smaenvelope::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::smaenvelope::indicator_by_options;

use crate::indicators::simd_indicators::sma_simd::calc_simd as calc_sma_simd;
use std::simd::Simd;

/// Computes one step of the SMA Envelope for `N` lanes simultaneously using SIMD parallelism.
///
/// Advances the rolling SMA sum by adding `value` and subtracting `prev_value`, computes
/// the SMA for every lane, then derives the lower and upper envelope bands.
///
/// # Arguments
///
/// * `sum` - Mutable SIMD vector of per-lane rolling window sums (updated in place).
/// * `value` - Current (newest) bar prices for all `N` lanes.
/// * `prev_value` - Prices leaving the rolling window for all `N` lanes.
/// * `multipliers` - Precomputed `(1/period, percentage/100)` SIMD vectors, broadcast
///   from the shared scalar multipliers.
///
/// # Returns
///
/// `(lower, middle, upper)` — SIMD vectors for all `N` lanes where `middle` is the SMA
/// and `lower`/`upper` are `middle ± middle * (percentage / 100)`.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    sum: &mut Simd<f64, N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let sma = calc_sma_simd(sum, value, prev_value, multipliers.0);
    let step = sma * multipliers.1;

    (sma - step, sma, sma + step)
}
