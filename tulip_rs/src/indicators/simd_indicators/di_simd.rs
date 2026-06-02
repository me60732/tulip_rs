use crate::indicators::di::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::di::indicator_by_assets;
use crate::indicators::simd_indicators::{
    atr_simd::partial_calc_simd as atr_partial_calc_simd, atr_simd::SimdState as AtrSimdState,
    dm_simd::calc_simd as dm_calc_simd, dm_simd::SimdState as DmSimdState,
    simd_types::F64Constants,
};

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::di::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, Select, Simd};

/// SIMD-parallel state for computing the Directional Indicator (DI) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    /// Underlying Directional Movement (DM) SIMD state tracking Wilder-smoothed +DM and -DM.
    pub di_state: DmSimdState<N>,
    /// Underlying Average True Range (ATR) SIMD state used to normalise the directional movement.
    pub atr_state: AtrSimdState<N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a `SimdState`, packing each sub-state
    /// (`di_state`, `atr_state`) into the corresponding SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        // Create vectors to collect the references
        let mut di_refs = Vec::with_capacity(N);
        let mut atr_refs = Vec::with_capacity(N);

        // Collect references and values
        for state in states.iter_mut() {
            di_refs.push(&mut state.di_state);
            atr_refs.push(&mut state.atr_state);
        }

        let di_state = DmSimdState::new(&di_refs);
        let atr_state = AtrSimdState::new(&atr_refs);

        Self {
            di_state,
            atr_state,
        }
    }

    /*pub fn to_states(&self) -> [State; N] {
        let di_states = self.di_state.to_states();
        let atr_states = self.atr_state.to_states();

        // Use into_iter() to consume the arrays and avoid move issues
        let states_vec: Vec<State> = short_states
            .into_iter()
            .zip(long_states.into_iter())
            .zip(prev_vidya.into_iter())
            .map(|((short_state, long_state), prev_vidya)| State {
                short_state,
                long_state,
                prev_vidya,
            })
            .collect();

        // Convert Vec to array
        states_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert states_vec to array"))
    }*/
    /// Writes the SIMD state back into `N` existing mutable [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut di_refs = Vec::with_capacity(N);
        let mut atr_refs = Vec::with_capacity(N);

        // Collect references and values
        for state in states.iter_mut() {
            di_refs.push(&mut state.di_state);
            atr_refs.push(&mut state.atr_state);
        }
        self.di_state.write_states(&mut di_refs);
        self.atr_state.write_states(&mut atr_refs);
    }
}

/// Advances the DI by one bar for `N` assets simultaneously.
///
/// Delegates to [`calc_diup_didown_simd`] for the Wilder-smoothed DM and ATR values, then
/// computes `+DI = 100 * dmup / atr` and `-DI = 100 * dmdown / atr`. Any NaN values
/// resulting from a zero ATR are replaced with `0.0`.
///
/// # Returns
///
/// A tuple `(plus_di, minus_di, atr, tr)` for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>)
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (dmup, dmdown, atr, tr) = calc_diup_didown_simd(state, high, low, close, multipliers);
    let atr_inv = F64Constants::HUNDRED / atr;
    let mut pdi = dmup * atr_inv; // multiplication
    let mut mdi = dmdown * atr_inv;

    // SIMD NaN detection and replacement
    pdi = pdi.simd_ne(pdi).select(F64Constants::ZERO, pdi); // if NaN, use 0, else use pdi

    mdi = mdi.simd_ne(mdi).select(F64Constants::ZERO, mdi); // if NaN, use 0, else use mdi

    (pdi, mdi, atr, tr)
}

/// Computes the Wilder-smoothed directional movement, ATR, and TR for `N` assets simultaneously.
///
/// Calls [`atr_partial_calc_simd`] for the current ATR and TR values, and [`dm_calc_simd`]
/// for the Wilder-smoothed +DM and -DM. Returns `(dmup, dmdown, atr, tr)` without the
/// percentage normalisation applied by [`calc_simd`].
#[inline(always)]
pub fn calc_diup_didown_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (atr, tr) = atr_partial_calc_simd(&mut state.atr_state, high, low, close, multipliers);
    let (dmup, dmdown) = dm_calc_simd(&mut state.di_state, high, low, multipliers.0);
    (dmup, dmdown, atr, tr)
}
