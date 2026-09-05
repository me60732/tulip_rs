#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::supersmoother::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::supersmoother::indicator_by_options;
use crate::indicators::supersmoother::State;

use std::simd::{Simd, StdFloat};
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for computing the Ehlers Super Smoother across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` holds the filter state for asset `i`.
pub struct SimdState<const N: usize> {
    pub y1: Simd<f64, N>,        // y[t-1] for each asset
    pub y2: Simd<f64, N>,        // y[t-2] for each asset
    pub prev_real: Simd<f64, N>, // x[t-1] for Ehlers input averaging
    pub a1: Simd<f64, N>,
    pub a2: Simd<f64, N>,
    pub b0: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_from_state!(
         sub: [],
         scalar: [y1, y2, prev_real, a1, a2, b0]
    );
    crate::simd_state_write!(
         sub: [],
         scalar: [y1, y2, prev_real]
    );

}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        real: Self::Inputs<'a>,
    ) -> Self::Outputs {
        // Ehlers: (b0/2) * (real + prev_real) + a1*y1 + a2*y2
        let y = self.b0.mul_add(real + self.prev_real, self.a1.mul_add(self.y1, self.a2 * self.y2));
        self.y2 = self.y1;
        self.y1 = y;
        self.prev_real = real;
        y
    }
}
