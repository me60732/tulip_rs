#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::bop::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::num::SimdFloat;
use std::simd::Simd;
//use crate::math_simd::fast_max
#[inline(always)]
pub fn calc_simd<const N: usize>(
    open: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N> {
    let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
    (close - open) / hl_diff
}
