use crate::indicators::highpass::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::highpass::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::highpass::indicator_by_options;

use std::simd::{Simd, StdFloat};
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for computing the Ehlers High Pass filter across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` holds the filter state for asset `i`.
pub struct SimdState<const N: usize> {
    pub y1: Simd<f64, N>,        // y[t-1] for each asset
    pub prev_real: Simd<f64, N>, // previous input price for each asset
    pub a1: Simd<f64, N>,
    pub a2: Simd<f64, N>
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;

    crate::simd_state_from_state!(
         sub: [],
         scalar: [y1, prev_real, a1, a2]
    );
    crate::simd_state_write!(
         sub: [],
         scalar: [y1, prev_real]
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
        let y = self.a1.mul_add(self.y1, self.a2 * (real - self.prev_real));
        self.prev_real = real;
        self.y1 = y;
        y
    }
}
