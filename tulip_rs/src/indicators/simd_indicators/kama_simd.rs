use crate::indicators::kama::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::kama::indicator_by_assets;
use crate::indicators::simd_indicators::{
    ef_simd::SimdState as EfSimdState, simd_types::F64Constants,
};

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::kama::indicator_by_options;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{Simd, StdFloat};
use crate::types::Warm;
/// SIMD-parallel state for computing the Kaufman Adaptive Moving Average (KAMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub ef_state: EfSimdState<N>,
    pub fast_ema:  Simd<f64, N>,
    pub slow_ema:  Simd<f64, N>,
    pub kama: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(ef_state: EfSimdState<N>)],
         scalar: [kama, fast_ema, slow_ema]
    );
    crate::simd_state_write!(
        sub: [(ef_state: EfSimdState<N>)],
        scalar: [kama]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (value, last_value): Self::Inputs<'a>
    ) -> Self::Outputs {

        let efficiency_ratio = self.ef_state.calc((value, last_value));

        let smoothing_constant = {
            let temp = (self.fast_ema - self.slow_ema).mul_add(efficiency_ratio, self.slow_ema);
            temp * temp // Square it by multiplying by itself
        };

        // Optimized calculation using C-style EMA pattern
        let per1 = F64Constants::ONE - smoothing_constant;
        //kama = kama * per1 + value * smoothing_constant;
        self.kama = self.kama.mul_add(per1, value * smoothing_constant);

        (self.kama, efficiency_ratio)
    }
}
