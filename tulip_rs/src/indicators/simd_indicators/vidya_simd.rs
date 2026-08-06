#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vidya::indicator_by_assets;
use crate::indicators::simd_indicators::stddev_simd::SimdState as StddevSimdState;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vidya::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::vidya::State;
use std::simd::{Simd, StdFloat};
use crate::types::Warm;
/// SIMD-parallel state for the Variable Index Dynamic Average (VIDYA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub short_state: StddevSimdState<N>,
    pub long_state: StddevSimdState<N>,
    alpha: Simd<f64, N>,
    prev_vidya: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(short_state: StddevSimdState<N>), (long_state: StddevSimdState<N>)],
         scalar: [prev_vidya, alpha]
    );
    crate::simd_state_write!(
         sub: [(short_state: StddevSimdState<N>), (long_state: StddevSimdState<N>)],
         scalar: [prev_vidya]
    );

}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (value, short_value, long_value): Self::Inputs<'a>
    ) -> Self::Outputs {

        let (sd_short, sma_short) = self.short_state.calc((value, short_value));

        // Compute long-term STDDEV.
        let (sd_long, sma_long) = self.long_state.calc((value, long_value));

        let mut k = sd_short / sd_long;
        k *= self.alpha;

        //self.prev_vidya = (value - self.prev_vidya) * k + self.prev_vidya;
        self.prev_vidya = (value - self.prev_vidya).mul_add(k, self.prev_vidya);
        (self.prev_vidya, sma_short, sma_long, sd_short, sd_long)
    }
}

