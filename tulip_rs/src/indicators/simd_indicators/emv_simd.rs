#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::emv::indicator_by_assets;

use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, *};
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
