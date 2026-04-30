pub use crate::indicators::simd_indicators::atr_simd::SimdState;
use crate::indicators::simd_indicators::simd_types::F64Constants;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::natr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::natr::indicator_by_options;
use std::simd::Simd;

impl<const N: usize> SimdState<N> {
    pub fn calc_natr_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (atr, tr) = self.calc_simd(high, low, close, multiplier);
        ((atr / close) * F64Constants::HUNDRED, atr, tr)
    }
}

