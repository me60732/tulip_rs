pub use crate::indicators::simd_indicators::stddev_simd::SimdState;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::bbands::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::bbands::indicator_by_options;

use std::simd::{Simd, StdFloat};
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    std_dev: Simd<f64, N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (sd, sma);
    (sd, sma) = state.calc_simd(value, prev_value, multiplier);

    //let upper_band = sma + std_dev * sd;
    let upper_band = std_dev.mul_add(sd, sma);
    //let lower_band = sma - std_dev * sd;
    let lower_band = (-std_dev).mul_add(sd, sma);
    (lower_band, sma, upper_band)
}
