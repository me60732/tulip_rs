use crate::indicators::fosc::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::fosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::fosc::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    simd_types::F64Constants, tsf_simd::SimdState as SimdTsfState,
};
use crate::types::Warm;
use std::simd::Simd;
/// SIMD-parallel state for computing the Forecast Oscillator (FOSC) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Underlying linear-regression / TSF SIMD state carrying the per-asset sum accumulators.
    tsf_state: SimdTsfState<N>,
    /// Most recent Time Series Forecast (TSF) value per asset lane, used on the next bar to compute FOSC.
    tsf: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
        sub: [(tsf_state: SimdTsfState<N>)],
        scalar: [tsf]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    );

    /// Computes one FOSC step across `N` asset lanes using SIMD parallelism.
    ///
    /// FOSC measures the percentage deviation of the current price from the Time Series
    /// Forecast: `fosc = 100 * (value - tsf_prev) / value`. It then advances the
    /// underlying linear-regression / TSF state so that `tsf_prev` is ready for the
    /// next bar.
    ///
    /// Returns `(fosc, tsf, linreg, slope, intercept)` for all `N` lanes simultaneously.
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (prev_value, value): Self::Inputs<'a>,
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let fosc = F64Constants::HUNDRED * (value - self.tsf) / value; //.max(f64::EPSILON);

        let (tsf, linreg, slope, intercept) = self.tsf_state.calc((prev_value, value));
        self.tsf = tsf;
        (fosc, tsf, linreg, slope, intercept)
    }
}
