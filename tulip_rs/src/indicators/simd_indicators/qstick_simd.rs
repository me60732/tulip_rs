#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::qstick::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::qstick::indicator_by_options;

use std::simd::Simd;

/// Computes one bar of the QStick indicator for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Updates the running sum of `(close - open)` by adding the new bar's body and
/// dropping the oldest bar's body, then returns `sum * multiplier` (the SMA of body lengths).
///
/// # Arguments
///
/// * `open` - Open prices for this bar.
/// * `close` - Close prices for this bar.
/// * `prev_open` - Open prices from `period` bars ago (the bar being dropped from the window).
/// * `prev_close` - Close prices from `period` bars ago.
/// * `sum` - Running window sum of `(close - open)`; updated in place.
/// * `multiplier` - Per-lane SMA normalisation factor `1 / period`.
///
/// # Returns
///
/// QStick values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    open: Simd<f64, N>,
    close: Simd<f64, N>,
    prev_open: Simd<f64, N>,
    prev_close: Simd<f64, N>,
    sum: &mut Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    let mut s = *sum;
    s += (close - open) - (prev_close - prev_open);
    *sum = s;
    s * multiplier
}
