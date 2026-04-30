use crate::indicators::simd_indicators::sma_simd::calc_simd as calc_sma_simd;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dpo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dpo::indicator_by_options;

use std::simd::Simd;

#[inline(always)]
pub fn calc_simd<const N: usize>(
    value: Simd<f64, N>,
    sum: &mut Simd<f64, N>,
    prev_values: (Simd<f64, N>, Simd<f64, N>),
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    //let (sma, mut s) = (0.0, *sum);
    let (prev_value, dpo_price) = prev_values;
    let sma = calc_sma_simd(sum, value, prev_value, multiplier);
    (dpo_price - sma, sma)
}
