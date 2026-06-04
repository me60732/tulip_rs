use crate::indicators::chandelierexit::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::chandelierexit::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::chandelierexit::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, 
    min_simd::SimdState as SimdMinState,
    atr_simd::SimdState as SimdAtrState
};
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for computing the Aroon indicator across `N` assets simultaneously.
/// Wraps dedicated min/max ring-buffer SIMD states for tracking the lookback window.
pub struct SimdState<const N: usize> {
    min_state: SimdMinState<N>,
    max_state: SimdMaxState<N>,
    atr_state: SimdAtrState<N>
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut min_state = Vec::with_capacity(N);
        let mut max_state = Vec::with_capacity(N);
        let mut atr_state = Vec::with_capacity(N);
        
        for state in states.iter_mut() {
            min_state.push(&mut state.min_state);
            max_state.push(&mut state.max_state);
            atr_state.push(&mut state.atr_state);
        }
        let min_state = SimdMinState::new(&min_state);
        let max_state = SimdMaxState::new(&max_state);
        let atr_state = SimdAtrState::new(&atr_state);

        Self {
            min_state,
            max_state,
            atr_state,
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut max_refs = Vec::with_capacity(N);
        let mut min_refs = Vec::with_capacity(N);
        let mut atr_refs = Vec::with_capacity(N);
        
        for state in states.iter_mut() {
            max_refs.push(&mut state.max_state);
            min_refs.push(&mut state.min_state);
            atr_refs.push(&mut state.atr_state);
        }
        self.max_state.write_states(&mut max_refs);
        self.min_state.write_states(&mut min_refs);
        self.atr_state.write_states(&mut atr_refs);
    }
}
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::Calc as CalcMax, min_simd::assets::Calc as CalcMin,
    };

    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: usize,
            look_back: usize,
            multipliers: (Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>)),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            high_ptrs: [*const f64; N],
            low_ptrs: [*const f64; N],
            close: Simd<f64, N>,
            i: usize,
            look_back: usize,
            multipliers: (Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>)),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (step, atr_multipliers) = multipliers;
            let (min, _) = self
                .min_state
                .calc_unchecked_simd::<WINDOW_LANES>(low_ptrs, i, look_back);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd::<WINDOW_LANES>(high_ptrs, i, look_back);

            let (high, low) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (atr, tr) = self.atr_state.calc_simd(high, low, close, atr_multipliers);

            let long = atr.mul_add(-step, max);
            let short = atr.mul_add(step, min);
            
            (long, short, atr, tr)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
            multipliers: (Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>)),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            high_ptrs: [*const f64; N],
            low_ptrs: [*const f64; N],
            close: Simd<f64, N>,
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
            multipliers: (Simd<f64, N>, (Simd<f64, N>, Simd<f64, N>)),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (step, atr_multipliers) = multipliers;
            let (min, _) = self
                .min_state
                .calc_unchecked_simd(low_ptrs, i, look_back);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd(high_ptrs, i, look_back);

            let (high, low) = crate::extract_simd_inputs_at_index_array!(i.as_array(), N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (atr, tr) = self.atr_state.calc_simd(high, low, close, atr_multipliers);

            let long = atr.mul_add(-step, max);
            let short = atr.mul_add(step, min);
            
            (long, short, atr, tr)
        }
    }
}
