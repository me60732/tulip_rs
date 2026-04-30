use crate::indicators::dm::State;
use crate::indicators::simd_indicators::simd_types::F64Constants;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dm::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dm::indicator_by_options;

use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
pub struct SimdState<const N: usize> {
    pub dmup: Simd<f64, N>,
    pub dmdown: Simd<f64, N>,
    pub prev_high: Simd<f64, N>,
    pub prev_low: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut dmup = [0.0; N];
        let mut dmdown = [0.0; N];
        let mut prev_high = [0.0; N];
        let mut prev_low = [0.0; N];

        for i in 0..N {
            dmup[i] = states[i].dmup;
            dmdown[i] = states[i].dmdown;
            prev_high[i] = states[i].prev_high;
            prev_low[i] = states[i].prev_low;
        }
        Self {
            dmup: Simd::from_array(dmup),
            dmdown: Simd::from_array(dmdown),
            prev_high: Simd::from_array(prev_high),
            prev_low: Simd::from_array(prev_low),
        }
    }
    /*pub fn to_states(&self) -> [State; N] {
        let atr = self.atr.to_array();
        let prev_close = self.prev_close.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(atr[i], prev_close[i]));

        states
    }*/
    pub fn write_states(&self, states: &mut [&mut State]) {
        let dmup = self.dmup.to_array();
        let dmdown = self.dmdown.to_array();
        let prev_high = self.prev_high.to_array();
        let prev_low = self.prev_low.to_array();

        for i in 0..N {
            states[i].dmup = dmup[i];
            states[i].dmdown = dmdown[i];
            states[i].prev_high = prev_high[i];
            states[i].prev_low = prev_low[i];
        }
    }
}
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let (dp, dm) = calc_dp_dm_simd(state, high, low);
    let (_, _) = calc_dmup_dmdown_simd(state, dp, dm, multiplier);
    (state.dmup, state.dmdown)
}

#[inline(always)]
fn calc_dmup_dmdown_simd<const N: usize>(
    state: &mut SimdState<N>,
    dp: Simd<f64, N>,
    dm: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.dmup = state.dmup.mul_add(multiplier, dp);
    state.dmdown = state.dmdown.mul_add(multiplier, dm);

    (state.dmup, state.dmdown)
}

/*#[inline(always)]
pub fn calc_dp_dm_simd1<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>)
{
    let mut dp = high - state.prev_high;
    let mut dm = state.prev_low - low;
    (state.prev_high, state.prev_low) = (high, low);

    // Block 1: if dp < 0 then dp = 0, else if dp > dm then dm = 0
    let dp_neg = dp.simd_lt(F64Constants::ZERO);
    dp = dp_neg.select(F64Constants::ZERO, dp);
    let dp_wins = (!dp_neg) & dp.simd_gt(dm);
    dm = dp_wins.select(F64Constants::ZERO, dm);

    // Block 2: if dm < 0 then dm = 0, else if dm > dp then dp = 0
    let dm_neg = dm.simd_lt(F64Constants::ZERO);
    dm = dm_neg.select(F64Constants::ZERO, dm);
    let dm_wins = (!dm_neg) & dm.simd_gt(dp);
    dp = dm_wins.select(F64Constants::ZERO, dp);

    (dp, dm)
}*/

#[inline(always)]
pub fn calc_dp_dm_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let mut dp = high - state.prev_high;
    let mut dm = state.prev_low - low;
    (state.prev_high, state.prev_low) = (high, low);

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
