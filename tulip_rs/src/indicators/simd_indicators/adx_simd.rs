use crate::indicators::adx::State;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adx::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adx::indicator_by_options;

use crate::indicators::simd_indicators::{
    dx_simd::{calc_simd as dx_calc_simd, SimdState as DxSimdState},
    wilders_simd::calc_simd as wilders_calc_simd,
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Average Directional Index (ADX) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Embedded DX SIMD state containing Directional Movement and ATR sub-states.
    pub dx_state: DxSimdState<N>,
    /// Running Wilder-smoothed ADX value for each asset.
    pub adx: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        // Create vectors to collect the references
        let mut dx_refs = Vec::with_capacity(N);
        let mut adx = [0.0; N];

        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            dx_refs.push(&mut state.dx_state);
            adx[i] = state.adx;
        }

        let dx_state = DxSimdState::new(&mut dx_refs);

        Self {
            dx_state,
            adx: Simd::from_array(adx),
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
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut dx_refs = Vec::with_capacity(N);
        let adx = self.adx.to_array();

        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            dx_refs.push(&mut state.dx_state);
            state.adx = adx[i];
        }
        self.dx_state.write_states(&mut dx_refs);
    }
}

/// Advances the Average Directional Index (ADX) by one bar for `N` assets simultaneously.
///
/// Delegates to [`dx_calc_simd`] to update the embedded DX / DM / ATR sub-states, then applies
/// Wilder smoothing to produce the ADX.
///
/// # Arguments
///
/// * `state` - Mutable SIMD state carrying the per-lane ADX and DX sub-states.
/// * `high` - High prices for the current bar across all `N` lanes.
/// * `low` - Low prices for the current bar across all `N` lanes.
/// * `close` - Close prices for the current bar across all `N` lanes.
/// * `multiplier` - Wilder smoothing constant, SIMD-broadcast across all lanes.
///
/// # Returns
///
/// A tuple `(adx, dx, atr, tr)` of SIMD vectors for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (dx, atr, tr) = dx_calc_simd(&mut state.dx_state, high, low, close, multiplier);
    state.adx = wilders_calc_simd(state.adx, dx, multiplier);
    (state.adx, dx, atr, tr)
}
