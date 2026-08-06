use crate::indicators::dm::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dm::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;
pub use crate::indicator_types::{TState, TSimdState};
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dm::indicator_by_options;
use crate::types::Warm;
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
/// SIMD-parallel state for the Directional Movement (DM) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub dmup: Simd<f64, N>,
    pub dmdown: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
    pub prev_high: Simd<f64, N>,
    pub prev_low: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
        sub: [],
        scalar: [dmup, dmdown, multiplier, prev_high, prev_low]
    );
    crate::simd_state_write!(
        sub: [],
        scalar: [dmup, dmdown, prev_high, prev_low]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let dp_dm = self.calc_dp_dm(inputs);
        let (dmup, dmdown) = self.calc_dmup_dmdown(dp_dm);
        (dmup, dmdown)
    }
}
impl<const N: usize> SimdState<N> {
    #[inline(always)]
    fn calc_dmup_dmdown(
        &mut self,
        (dp, dm): (Simd<f64, N>, Simd<f64, N>)
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        self.dmup = self.dmup.mul_add(self.multiplier, dp);
        self.dmdown = self.dmdown.mul_add(self.multiplier, dm);
    
        (self.dmup, self.dmdown)
    }
    #[inline(always)]
    pub fn calc_dp_dm(
        &mut self,
        (high, low): (Simd<f64, N>, Simd<f64, N>)
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let mut dp = high - self.prev_high;
        let mut dm = self.prev_low - low;
        (self.prev_high, self.prev_low) = (high, low);
    
        // Clamp to non-negative (simd_max is cleaner for one-sided clamp)
        dp = dp.simd_max(F64Constants::ZERO);
        dm = dm.simd_max(F64Constants::ZERO);
    
        // Mutual exclusion: zero the loser
        let dp_wins = dp.simd_gt(dm);
        dm = dp_wins.select(F64Constants::ZERO, dm);
    
        let dm_wins = dm.simd_gt(dp);
        dp = dm_wins.select(F64Constants::ZERO, dp);
    
        (dp, dm)
    }

}

