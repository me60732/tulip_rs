#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::trima::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::trima::indicator_by_options;

use crate::indicators::trima::State;
use std::simd::Simd;
use crate::types::Warm;
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for computing the Triangular Moving Average (TRIMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Weighted running sum (numerator of the current TRIMA) for each lane.
    pub weight_sum: Simd<f64, N>,
    /// Running sum of values entering the leading half of the triangular window for each lane.
    pub lead_sum: Simd<f64, N>,
    /// Running sum of values in the trailing half of the triangular window for each lane.
    pub trail_sum: Simd<f64, N>,
    pub multiplier: Simd<f64, N>
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [],
         scalar: [weight_sum, lead_sum, trail_sum, multiplier]
    );
    crate::simd_state_write!(
         sub: [],
         scalar: [weight_sum, lead_sum, trail_sum]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    fn calc<'a>(
        &mut self,
        (real, lsi, tsi1, tsi2): Self::Inputs<'a>,
    ) -> Self::Outputs {
        //calc_simd(self, real, lsi, tsi1, tsi2, multiplier)
        self.weight_sum += real;
        let trima = self.weight_sum * self.multiplier;
        self.lead_sum += real;
        self.weight_sum += self.lead_sum - self.trail_sum;
        self.lead_sum -= lsi;
        self.trail_sum += tsi1 - tsi2;

        trima
    }
}

