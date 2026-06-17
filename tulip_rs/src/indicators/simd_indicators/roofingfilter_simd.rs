use crate::indicators::roofingfilter::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::roofingfilter::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::roofingfilter::indicator_by_options;

use crate::indicators::simd_indicators::{
    supersmoother_simd::SimdState as SimdSSState,
    highpass_simd::SimdState as SimdHPState
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Ehlers Roofing Filter across `N` assets simultaneously.
/// Each field holds the packed SIMD state for the two cascaded sub-filters:
/// a HighPass filter followed by a SuperSmoother.
pub struct SimdState<const N: usize> {
    ss_state: SimdSSState<N>,
    hp_state: SimdHPState<N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`],
    /// packing `ss_state` and `hp_state` from each asset into their respective SIMD sub-states.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut ss_state = Vec::with_capacity(N);
        let mut hp_state = Vec::with_capacity(N);

        for state in states.iter_mut() {
            ss_state.push(&mut state.ss_state);
            hp_state.push(&mut state.hp_state);
        }
        let ss_state = SimdSSState::new(ss_state.as_slice());
        let hp_state = SimdHPState::new(hp_state.as_slice());
        
        Self {
            ss_state,
            hp_state
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// scattering each lane's `ss_state` and `hp_state` back to its corresponding asset.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut ss_refs = Vec::with_capacity(N);
        let mut hp_refs = Vec::with_capacity(N);

        // Collect references and values
        for state in states.iter_mut() {
            ss_refs.push(&mut state.ss_state);
            hp_refs.push(&mut state.hp_state);
        }
        self.ss_state.write_states(&mut ss_refs);
        self.hp_state.write_states(&mut hp_refs);
    }
    /// Advances the Roofing Filter by one bar across all `N` assets simultaneously.
    ///
    /// First applies the HighPass filter to `real`, then passes the result through
    /// the SuperSmoother, matching the scalar `State::calc` logic.
    ///
    /// # Arguments
    ///
    /// * `real` - SIMD vector of current input prices, one per asset lane.
    /// * `multipliers` - Tuple of SIMD coefficient vectors `((a1, a2, b0), (a1, a2))`,
    ///   broadcast from [`crate::indicators::roofingfilter::multiplier`].
    ///
    /// # Returns
    ///
    /// A tuple `(roofing, highpass)` of SIMD vectors, one value per asset lane.
    #[inline(always)]
    pub fn calc_simd(&mut self, real: Simd<f64, N>, multipliers: ((Simd<f64, N>, Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>))) -> (Simd<f64, N>, Simd<f64, N>) {
        let hp = self.hp_state.calc_simd(real, multipliers.1);
        (self.ss_state.calc_simd(hp, multipliers.0), hp)
    }
}

