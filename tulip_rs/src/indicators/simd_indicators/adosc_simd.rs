pub use crate::indicator_types::{TState, TSimdState};
use crate::indicators::adosc::State as State;
use crate::indicators::simd_indicators::{
    ad_simd::SimdState as AdSimdState, ema_simd::SimdState as EmaSimdState,
};
use std::simd::Simd;
use crate::types::Warm;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adosc::indicator_by_options;

/// SIMD-parallel state for computing the Chaikin AD Oscillator (ADOSC) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub short_ema: EmaSimdState<N>,
    pub long_ema: EmaSimdState<N>,
    pub ad: AdSimdState<N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut ad_refs = Vec::with_capacity(N);
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];
        let mut short_multiplier = [0.0; N];
        let mut long_multiplier = [0.0; N];
        let mut short_inv_multiplier = [0.0; N];
        let mut long_inv_multiplier = [0.0; N];

        for (i, state) in states.iter_mut().enumerate() {
            let [short, long] = state.ema_state.ema.to_array();
            let [short_m, long_m] = state.ema_state.multiplier.to_array();
            let [short_im, long_im] = state.ema_state.inv_multiplier.to_array();
            ad_refs.push(&mut state.ad_state);
            short_ema[i] = short;
            long_ema[i] = long;

            short_multiplier[i] = short_m;
            long_multiplier[i] = long_m;
            short_inv_multiplier[i] = short_im;
            long_inv_multiplier[i] = long_im;
            long_ema[i] = long;
        }
        let ad =  AdSimdState::from_states(&mut ad_refs);
        Self {
            ad,
            short_ema: EmaSimdState::new(
                Simd::from_array(short_ema),
                (
                    Simd::from_array(short_multiplier),
                    Simd::from_array(short_inv_multiplier),
                ),
            ),
            long_ema: EmaSimdState::new(
                Simd::from_array(long_ema),
                (
                    Simd::from_array(long_multiplier),
                    Simd::from_array(long_inv_multiplier),
                ),
            ),
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to [`to_states`].
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let short_ema = self.short_ema.ema.to_array();
        let long_ema = self.long_ema.ema.to_array();
        let mut ad_refs = Vec::with_capacity(N);
        
        for (i, state) in states.iter_mut().enumerate() {
            let [short, long] = state.ema_state.ema.as_mut_array();
            *short = short_ema[i];
            *long = long_ema[i];
            ad_refs.push(&mut state.ad_state);
        }
        self.ad.write_states(&mut ad_refs);
    }
    
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>,
    ) -> Self::Outputs {

        let ad = self.ad.calc(inputs);
        let short_ema = self.short_ema.calc(ad);
        let long_ema = self.long_ema.calc(ad);

        short_ema - long_ema
    }
}
