pub use crate::indicators::simd_indicators::linreg_simd::SimdState;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tsf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::tsf::indicator_by_options;

use crate::indicators::simd_indicators::{
    linreg_simd::calc_simd as linreg_calc_simd, simd_types::F64Constants,
};
use std::simd::{Simd, StdFloat};
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    prev_value: Simd<f64, N>,
    value: Simd<f64, N>,
    period: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (linreg, slope, intercept);
    (linreg, slope, intercept) = linreg_calc_simd(state, prev_value, value, period);
    //let tsf = intercept + slope * (period + F64Constants::ONE);
    let tsf = slope.mul_add(period + F64Constants::ONE, intercept);
    (tsf, linreg, slope, intercept)
}
