#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::emv::indicator_by_assets;

use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, *};
/// Computes one bar of the Ease of Movement (EMV) indicator for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Calculates `(midpoint_move * 10000 * (high - low)) / volume`, where midpoint move is
/// the change in median price from the previous bar.
/// Volume and the high-low range are clamped to [`F64Constants::EPSILON`] to avoid division by zero.
///
/// # Arguments
///
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `volume` - Volume for this bar.
/// * `prev_medprice` - Previous bar's median price for each lane; updated in place.
///
/// # Returns
///
/// EMV values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    volume: Simd<f64, N>,
    prev_medprice: &mut Simd<f64, N>,
) -> Simd<f64, N> {
    let medprice = calc_medprice_simd(high, low);
    let distance_moved = medprice - *prev_medprice;
    let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
    let volume_safe = volume.simd_max(F64Constants::EPSILON);
    *prev_medprice = medprice;

    distance_moved * F64Constants::TEN_THOUSAND * hl_diff / volume_safe
}
