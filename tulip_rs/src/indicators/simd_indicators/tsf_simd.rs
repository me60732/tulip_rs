pub use crate::indicators::simd_indicators::linreg_simd::SimdState;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tsf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::tsf::indicator_by_options;

use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};
pub trait Calc<const N: usize> {
    fn calc_simd(
        &mut self,
        prev_value: Simd<f64, N>,
        value: Simd<f64, N>,
        period: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
}
impl<const N: usize> Calc<N> for SimdState<N> {
    /// Computes one bar of the Time Series Forecast (TSF) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Delegates to the linear-regression SIMD routine and projects one period forward:
    /// `tsf = intercept + slope * (period + 1)`.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable SIMD state from the underlying linear-regression calculation.
    /// * `prev_value` - Oldest price being dropped from the regression window.
    /// * `value` - Current prices for this bar.
    /// * `period` - Look-back period as a SIMD vector (same value in each lane for assets mode).
    ///
    /// # Returns
    ///
    /// A tuple `(tsf, linreg, slope, intercept)` for all `N` lanes.
    #[inline(always)]
    fn calc_simd(
        &mut self,
        prev_value: Simd<f64, N>,
        value: Simd<f64, N>,
        period: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (linreg, slope, intercept);
        (linreg, slope, intercept) = SimdState::calc_simd(self, prev_value, value, period);
        //let tsf = intercept + slope * (period + F64Constants::ONE);
        let tsf = slope.mul_add(period + F64Constants::ONE, intercept);
        (tsf, linreg, slope, intercept)
    }
}