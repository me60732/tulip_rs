#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::elderray::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::elderray::indicator_by_options;
use crate::indicators::simd_indicators::ema_simd::calc_simd as calc_ema_simd;
pub use crate::indicators::simd_indicators::ema_simd::multiplier_simd;
use std::simd::Simd;

/// Computes one bar of Elder-ray for `N` lanes simultaneously using SIMD parallelism.
///
/// Advances the EMA by one bar and returns the bull power, bear power, and updated EMA
/// for all `N` lanes in a single pass.
///
/// # Arguments
///
/// * `high` - High prices for this bar across all `N` lanes.
/// * `low` - Low prices for this bar across all `N` lanes.
/// * `close` - Close prices for this bar across all `N` lanes.
/// * `prev_ema` - Previous EMA values for each lane.
/// * `multipliers` - Tuple `(multiplier, inv_multiplier)` from [`multiplier_simd`].
///
/// # Returns
///
/// A tuple `(bull, bear, ema)` where:
/// * `bull` = `high − EMA` for each lane
/// * `bear` = `low − EMA` for each lane
/// * `ema` = updated EMA values for each lane
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    prev_ema: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let ema = calc_ema_simd(close, prev_ema, multipliers);

    (high - ema, low - ema, ema)
}
