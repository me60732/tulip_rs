use crate::indicators::simd_indicators::sma_simd::calc_simd as calc_sma_simd;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dpo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dpo::indicator_by_options;

use std::simd::Simd;

/// Computes one bar of the Detrended Price Oscillator (DPO) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Calculates the SMA over the current window, then returns `dpo_price - sma` where
/// `dpo_price` is the historical close offset `period/2 + 1` bars ago.
///
/// # Arguments
///
/// * `value` - Current close prices for this bar.
/// * `sum` - Running sum used by the underlying SMA calculation; updated in place.
/// * `prev_values` - Tuple of `(prev_value, dpo_price)`: the oldest value dropped from the SMA
///   window and the historical close used as the DPO reference price.
/// * `multiplier` - Per-lane SMA normalisation factor `1 / period`.
///
/// # Returns
///
/// A tuple `(dpo, sma)` for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    value: Simd<f64, N>,
    sum: &mut Simd<f64, N>,
    prev_values: (Simd<f64, N>, Simd<f64, N>),
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    //let (sma, mut s) = (0.0, *sum);
    let (prev_value, dpo_price) = prev_values;
    let sma = calc_sma_simd(sum, value, prev_value, multiplier);
    (dpo_price - sma, sma)
}
