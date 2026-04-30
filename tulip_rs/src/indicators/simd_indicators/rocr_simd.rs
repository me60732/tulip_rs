#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::rocr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::rocr::indicator_by_options;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{num::SimdFloat, *};

#[inline(always)]
pub fn calc_simd<const N: usize>(real: Simd<f64, N>, prev_real: Simd<f64, N>) -> Simd<f64, N> {
    real / prev_real.simd_max(F64Constants::EPSILON)
}
