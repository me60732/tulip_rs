#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::medprice::indicator_by_assets;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::Simd;

/// Computes the Median Price across `N` asset lanes simultaneously.
///
/// `medprice = (high + low) / 2`. This is a stateless, per-bar calculation with no lookback period.
#[inline(always)]
pub fn calc_simd<const N: usize>(high: Simd<f64, N>, low: Simd<f64, N>) -> Simd<f64, N> {
    //(high + low) / 2.0 // * 0.5
    F64Constants::HALF * (high + low) // * 0.5
}
