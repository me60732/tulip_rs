use crate::indicators::simd_indicators::{simd_types::F64Constants, tema_simd::calc_simd as tema_calc_simd};
use std::simd::Simd;

pub use crate::indicators::simd_indicators::tema_simd::SimdState;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trix::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trix::indicator_by_options;

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    multiplier: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let prev_ema3 = state.ema3;
    let (tema, dema, ema) = tema_calc_simd(state, value, multiplier);
    // Compute TRIX as percentage change if previous TEMA is non-zero.
    let trix = F64Constants::HUNDRED * (state.ema3 - prev_ema3) / state.ema3;
    (trix, tema, dema, ema)
}
