use crate::indicators::di::State;
use crate::indicators::simd_indicators::{
    atr_simd::partial_calc_simd as atr_partial_calc_simd, atr_simd::SimdState as AtrSimdState,
    dm_simd::calc_simd as dm_calc_simd, dm_simd::SimdState as DmSimdState,
    simd_types::F64Constants,
};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::di::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::di::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, Select, Simd};

pub struct SimdState<const N: usize> {
    pub di_state: DmSimdState<N>,
    pub atr_state: AtrSimdState<N>,
}
impl<const N: usize> SimdState<N> {
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

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (dmup, dmdown, atr, tr) = calc_diup_didown_simd(state, high, low, close, multiplier);
    let atr_inv = F64Constants::HUNDRED / atr;
    let mut pdi = dmup * atr_inv; // multiplication
    let mut mdi = dmdown * atr_inv;

    // SIMD NaN detection and replacement
    pdi = pdi.simd_ne(pdi).select(F64Constants::ZERO, pdi); // if NaN, use 0, else use pdi

    mdi = mdi.simd_ne(mdi).select(F64Constants::ZERO, mdi); // if NaN, use 0, else use mdi

    (pdi, mdi, atr, tr)
}

#[inline(always)]
pub fn calc_diup_didown_simd<const N: usize>(
    state: &mut SimdState<N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let (atr, tr) = atr_partial_calc_simd(&mut state.atr_state, high, low, close, multiplier);
    let (dmup, dmdown) = dm_calc_simd(&mut state.di_state, high, low, multiplier);
    (dmup, dmdown, atr, tr)
}
