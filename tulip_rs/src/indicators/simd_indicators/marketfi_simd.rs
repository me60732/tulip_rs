#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::marketfi::indicator_by_assets;

use std::simd::{num::SimdFloat, *};

use crate::indicators::simd_indicators::simd_types::F64Constants;
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    volume: Simd<f64, N>,
) -> Simd<f64, N> {
    (high - low) / volume.simd_max(F64Constants::EPSILON)
}
