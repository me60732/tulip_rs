use crate::indicators::linreg::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::linreg::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::linreg::indicator_by_options;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{Simd, StdFloat};
use crate::types::Warm;
/// SIMD-parallel state for computing Linear Regression across `N` assets/options simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset/option `i`.
pub struct SimdState<const N: usize> {
    /// Running sum of x (time-index) values — precomputed and constant for a given period.
    pub sum_x: Simd<f64, N>,
    /// Running sum of y (price) values over the current window.
    pub sum_y: Simd<f64, N>,
    /// Running sum of x*y cross-products over the current window.
    pub sum_xy: Simd<f64, N>,
    /// Precomputed denominator `1 / (period * sum_x^2 - sum_x^2)` used each bar.
    pub per: Simd<f64, N>,
    pub n: Simd<f64, N>
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [],
         scalar: [sum_x, sum_y, sum_xy, per, n]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    /// Computes one linear regression step across `N` lanes using SIMD parallelism.
    ///
    /// Maintains running sums `sum_xy` and `sum_y` using a sliding-window update:
    /// new value is added, oldest is evicted via `prev_value`. Computes slope,
    /// intercept, and the end-point `linreg` value using FMA for each lane.
    ///
    /// Returns `(linreg, slope, intercept)`.
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (prev_value, value): Self::Inputs<'a>
    ) -> Self::Outputs {
        let (sum_x, mut sum_y, mut sum_xy, per, period) = (self.sum_x, self.sum_y, self.sum_xy, self.per, self.n);
    
        // FMA: (value * period) + sum_xy
        sum_xy = value.mul_add(period, sum_xy);
        sum_y += value;
    
        // slope = (period * sum_xy - sum_x * sum_y) * per
        let slope = sum_x.mul_add(-sum_y, period * sum_xy) * per;
    
        // intercept = (sum_y - slope * sum_x) / period
        let intercept = slope.mul_add(-sum_x, sum_y) / period;
    
        // linreg = intercept + slope * period
        let linreg = slope.mul_add(period, intercept);
    
        sum_xy -= sum_y;
        sum_y -= prev_value;
    
        (self.sum_y, self.sum_xy) = (sum_y, sum_xy);
        (linreg, slope, intercept)
    }
}


