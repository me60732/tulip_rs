#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::marketfi::indicator_by_assets;

use std::simd::{num::SimdFloat, *};

use crate::indicators::simd_indicators::simd_types::F64Constants;
/// Computes the Market Facilitation Index (MFI) across `N` asset lanes simultaneously.
///
/// MFI = (High - Low) / Volume. Volume is clamped to `EPSILON` to prevent division by zero.
/// This is a stateless, per-bar calculation with no lookback period.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    volume: Simd<f64, N>,
) -> Simd<f64, N> {
    (high - low) / volume.simd_max(F64Constants::EPSILON)
}
