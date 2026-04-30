#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::qstick::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::qstick::indicator_by_options;

use std::simd::Simd;

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
