#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::roc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::roc::indicator_by_options;

use crate::indicators::simd_indicators::{
    mom_simd::calc_simd as calc_mom_simd, rocr_simd::calc_simd as calc_rocr_simd,
};
use std::simd::Simd;

#[inline(always)]
pub fn calc_simd<const N: usize>(
    real: Simd<f64, N>,
    prev_real: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let mom = calc_mom_simd(real, prev_real);
    (calc_rocr_simd(mom, prev_real), mom)
}
