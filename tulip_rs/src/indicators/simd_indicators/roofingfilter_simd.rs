use crate::indicators::roofingfilter::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::roofingfilter::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::roofingfilter::indicator_by_options;

use crate::indicators::simd_indicators::{
    supersmoother_simd::SimdState as SimdSSState,
    highpass_simd::SimdState as SimdHPState
};
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::Simd;

/// SIMD-parallel state for computing the Ehlers Roofing Filter across `N` assets simultaneously.
/// Each field holds the packed SIMD state for the two cascaded sub-filters:
/// a HighPass filter followed by a SuperSmoother.
pub struct SimdState<const N: usize> {
    hp_state: SimdHPState<N>,
    ss_state: SimdSSState<N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    
    crate::simd_state_impl!(
         sub: [(ss_state: SimdSSState<N>), (hp_state: SimdHPState<N>)],
         scalar: []
    );

}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
        let hp = self.hp_state.calc(real);
        (self.ss_state.calc(hp), hp)
    }
}

