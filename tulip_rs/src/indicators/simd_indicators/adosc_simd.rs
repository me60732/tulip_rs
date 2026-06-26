use crate::indicators::adosc::IndicatorState as State;
use crate::indicators::simd_indicators::{
    ad_simd::calc_simd as calc_ad_simd, ema_simd::calc_simd as calc_ema_simd,
};
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adosc::indicator_by_options;

/// SIMD-parallel state for computing the Chaikin AD Oscillator (ADOSC) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Running Accumulation/Distribution line value for each asset.
    pub ad: Simd<f64, N>,
    /// Short-period EMA of the AD line for each asset.
    pub short_ema: Simd<f64, N>,
    /// Long-period EMA of the AD line for each asset.
    pub long_ema: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut ad = [0.0; N];
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];

        for i in 0..N {
            let [short, long] = states[i].ema_state.ema.to_array();
            ad[i] = states[i].ad;
            short_ema[i] = short;
            long_ema[i] = long;
        }
        Self {
            ad: Simd::from_array(ad),
            short_ema: Simd::from_array(short_ema),
            long_ema: Simd::from_array(long_ema),
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to [`to_states`].
    pub fn write_states(&self, states: &mut [&mut State]) {
        let ad = self.ad.to_array();
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        for i in 0..N {
            let [short, long] = states[i].ema_state.ema.as_mut_array();
            states[i].ad = ad[i];
            *short = short_ema[i];
            *long = long_ema[i];
            
        }
    }
    /// Advances the Chaikin AD Oscillator (ADOSC) by one bar for `N` assets simultaneously.
    ///
    /// Updates the AD line, then applies short- and long-period EMA smoothing. The oscillator value
    /// is the difference between the two EMAs (`short_ema - long_ema`).
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable SIMD state holding per-asset AD, short EMA, and long EMA.
    /// * `inputs` - Tuple of `(high, low, close, volume)` SIMD vectors for the current bar.
    /// * `multipliers` - Tuple of `(short_multiplier, long_multiplier)` EMA smoothing factors,
    ///   each itself a `(per, inv_per)` pair.
    ///
    /// # Returns
    ///
    /// ADOSC values (`short_ema - long_ema`) for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(
       &mut self,
        inputs: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
    ) -> Simd<f64, N> {
        let (high, low, close, volume) = inputs;
        let (short_multiplier, long_multiplier) = multipliers;
    
        self.ad = calc_ad_simd(self.ad, high, low, close, volume);
        self.short_ema = calc_ema_simd(self.ad, self.short_ema, short_multiplier);
        self.long_ema = calc_ema_simd(self.ad, self.long_ema, long_multiplier);
    
        self.short_ema - self.long_ema
    }
}


