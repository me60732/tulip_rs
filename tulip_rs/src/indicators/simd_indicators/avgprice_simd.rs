#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::avgprice::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, };

#[inline(always)]
pub fn calc_simd<const N: usize>(
    open: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N>
{
    (open + high + low + close) * F64Constants::QUATER
}
