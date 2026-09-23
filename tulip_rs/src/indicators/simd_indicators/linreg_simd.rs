use crate::indicators::linreg::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::linreg::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::linreg::indicator_by_options;
use crate::types::Warm;
use std::simd::{Simd, StdFloat};
/// SIMD-parallel state for computing Linear Regression across `N` assets/options simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset/option `i`.
pub struct SimdState<const N: usize> {
    pub sum_y: Simd<f64, N>,
    pub sum_xy: Simd<f64, N>,
    pub n: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_write!(
        sub: [],
        scalar: [sum_y, sum_xy]
    );
    crate::simd_state_from_state!(
        sub: [],
        scalar: [sum_y, sum_xy, n]
    );
    
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>));
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    /// Computes one linear regression step across `N` lanes using SIMD parallelism.
    ///
    /// Maintains running sums `sum_xy` and `sum_y` using a sliding-window update:
    /// new value is added, oldest is evicted via `prev_value`. Computes slope,
    /// intercept, and the end-point `linreg` value using FMA for each lane.
    ///
    /// Returns `(linreg, slope, intercept)`.
    #[inline(always)]
    fn calc<'a>(&mut self, (prev_value, value, (sum_x, per, inv_n)): Self::Inputs<'a>) -> Self::Outputs {
        // FMA: (value * period) + sum_xy
        self.sum_xy = value.mul_add(self.n, self.sum_xy);
        self.sum_y += value;

        // slope = (period * sum_xy - sum_x * sum_y) * per
        let slope = self.n.mul_add(self.sum_xy, -(sum_x * self.sum_y)) * per;
        let intercept = (-slope).mul_add(sum_x, self.sum_y) * inv_n;
        // linreg = intercept + slope * period
        let linreg = self.n.mul_add(slope, intercept);

        self.sum_xy -= self.sum_y;
        self.sum_y -= prev_value;

        (linreg, slope, intercept)
    }
}
impl<const N: usize> SimdState<N> {
    #[inline(always)]
    pub fn partial_calc(
        &mut self,
        (prev_value, value, (xy_coef, y_coef)): (Simd<f64, N>, Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>)),
    ) -> Simd<f64, N> {
        self.sum_xy = value.mul_add(self.n, self.sum_xy);
        self.sum_y += value;
        let linreg = self.sum_xy.mul_add(xy_coef, self.sum_y * y_coef);
        self.sum_xy -= self.sum_y;
        self.sum_y -= prev_value;
        linreg
    }
}
