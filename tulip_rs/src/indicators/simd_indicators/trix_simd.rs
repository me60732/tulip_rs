use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trix::indicator_by_assets;
pub use crate::indicators::simd_indicators::tema_simd::{SimdState, Calc as TemaCalc};

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trix::indicator_by_options;

pub trait Calc<const N: usize> {
    fn calc_simd(
        &mut self,
        value: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
}
impl<const N: usize> Calc<N> for SimdState<N> {
    /// Computes one bar of the Triple Exponential Average (TRIX) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Delegates to the TEMA SIMD routine for the triple-smoothed EMA, then computes
    /// `TRIX = 100 * (ema3 - prev_ema3) / ema3` as a percentage rate of change.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable SIMD state holding the three EMA stages.
    /// * `value` - Current prices for this bar.
    /// * `multiplier` - EMA multiplier pair `(per, 1 - per)`.
    ///
    /// # Returns
    ///
    /// A tuple `(trix, tema, dema, ema)` for all `N` lanes.
    #[inline(always)]
    fn calc_simd(
        &mut self,
        value: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let prev_ema3 = self.ema3;
        let (tema, dema, ema) = TemaCalc::calc_simd(self, value, multiplier);
        // Compute TRIX as percentage change if previous TEMA is non-zero.
        let trix = F64Constants::HUNDRED * (self.ema3 - prev_ema3) / self.ema3;
        (trix, tema, dema, ema)
    }
}
