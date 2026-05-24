#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::rocr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::rocr::indicator_by_options;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{num::SimdFloat, *};

/// Computes the Rate of Change Ratio (ROCR) for `N` asset lanes simultaneously.
///
/// ROCR is defined as `real / prev_real`, the ratio of the current price to the price
/// `n` periods ago. Division by zero is guarded by clamping the denominator to
/// [`f64::EPSILON`] before dividing.
///
/// # Returns
///
/// A SIMD vector of `N` lanes where lane `i` holds `real[i] / max(prev_real[i], ε)`.
#[inline(always)]
pub fn calc_simd<const N: usize>(real: Simd<f64, N>, prev_real: Simd<f64, N>) -> Simd<f64, N> {
    real / prev_real.simd_max(F64Constants::EPSILON)
}
