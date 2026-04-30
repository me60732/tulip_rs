#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tr::indicator_by_assets;
use std::simd::{num::SimdFloat, Simd};
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    prev_close: Simd<f64, N>,
) -> Simd<f64, N> {
    let hc = (high - prev_close).abs();
    let lc = (low - prev_close).abs();
    let hl = high - low;

    // True Range is the maximum of these three values
    hl.simd_max(hc).simd_max(lc)
}
