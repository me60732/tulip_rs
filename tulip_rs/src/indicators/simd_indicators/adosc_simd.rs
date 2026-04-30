use crate::indicators::adosc::State;
use crate::indicators::simd_indicators::{
    ad_simd::calc_simd as calc_ad_simd, ema_simd::calc_simd as calc_ema_simd,
};
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adosc::indicator_by_options;

pub struct SimdState<const N: usize> {
    pub ad: Simd<f64, N>,
    pub short_ema: Simd<f64, N>,
    pub long_ema: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut ad = [0.0; N];
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];

        for i in 0..N {
            ad[i] = states[i].ad;
            short_ema[i] = states[i].short_ema;
            long_ema[i] = states[i].long_ema;
        }
        Self {
            ad: Simd::from_array(ad),
            short_ema: Simd::from_array(short_ema),
            long_ema: Simd::from_array(long_ema),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let ad = self.ad.to_array();
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(ad[i], short_ema[i], long_ema[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let ad = self.ad.to_array();
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        for i in 0..N {
            states[i].ad = ad[i];
            states[i].short_ema = short_ema[i];
            states[i].long_ema = long_ema[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    inputs: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
) -> Simd<f64, N> {
    let (high, low, close, volume) = inputs;
    let (short_multiplier, long_multiplier) = multipliers;

    state.ad = calc_ad_simd(state.ad, high, low, close, volume);
    state.short_ema = calc_ema_simd(state.ad, state.short_ema, short_multiplier);
    state.long_ema = calc_ema_simd(state.ad, state.long_ema, long_multiplier);

    state.short_ema - state.long_ema
}
