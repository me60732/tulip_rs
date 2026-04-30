#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wcprice::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};

#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
) -> Simd<f64, N> {
    //(high + low + F64Constants::TWO * close) * F64Constants::QUATER
    close.mul_add(F64Constants::TWO, high + low) * F64Constants::QUATER
}
