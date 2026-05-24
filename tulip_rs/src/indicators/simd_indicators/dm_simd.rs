use crate::indicators::dm::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::dm::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::dm::indicator_by_options;

use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
/// SIMD-parallel state for the Directional Movement (DM) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub dmup: Simd<f64, N>,
    pub dmdown: Simd<f64, N>,
    pub prev_high: Simd<f64, N>,
    pub prev_low: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
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
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
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
/// Computes one bar of the Directional Movement (DM) indicator for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Advances the smoothed DM+ (`dmup`) and DM- (`dmdown`) running sums by one bar
/// using the Wilder smoothing formula: `dm = dm * multiplier + raw_dp_or_dm`.
///
/// # Arguments
///
/// * `state` - Mutable SIMD state holding current `dmup`, `dmdown`, `prev_high`, and `prev_low`.
/// * `high` - High prices for this bar.
/// * `low` - Low prices for this bar.
/// * `multiplier` - Per-lane Wilder smoothing decay factor `(1 - 1/period)`.
///
/// # Returns
///
/// A tuple `(dmup, dmdown)` of updated smoothed DM+ and DM- values for all `N` lanes.
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

/// Computes the raw positive (DP) and negative (DM) directional movement for `N` lanes.
///
/// Both values are clamped to non-negative, and the smaller of the two is zeroed out
/// so that only the dominant direction contributes on each bar.
/// Updates `state.prev_high` and `state.prev_low` in place.
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
