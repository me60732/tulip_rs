pub use crate::indicators::simd_indicators::atr_simd::SimdState;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::natr::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::natr::indicator_by_options;
use std::simd::Simd;

impl<const N: usize> SimdState<N> {
    /// Computes one bar of the Normalized Average True Range (NATR) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Computes the ATR via Wilder smoothing and normalizes it as `(atr / close) * 100`.
    ///
    /// # Arguments
    ///
    /// * `high` - High prices for this bar.
    /// * `low` - Low prices for this bar.
    /// * `close` - Close prices for this bar.
    /// * `multiplier` - Per-lane Wilder smoothing decay factor.
    ///
    /// # Returns
    ///
    /// A tuple `(natr, atr, tr)` for all `N` lanes.
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
