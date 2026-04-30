use crate::indicators::macd::State;
use crate::indicators::simd_indicators::ema_simd::calc_simd as calc_ema_simd;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::macd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::macd::indicator_by_options;

use std::simd::Simd;

pub struct SimdState<const N: usize> {
    pub short_ema: Simd<f64, N>,
    pub long_ema: Simd<f64, N>,
    pub signal: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];
        let mut signal = [0.0; N];

        for i in 0..N {
            short_ema[i] = states[i].short_ema;
            long_ema[i] = states[i].long_ema;
            signal[i] = states[i].signal;
        }

        Self {
            short_ema: Simd::from_array(short_ema),
            long_ema: Simd::from_array(long_ema),
            signal: Simd::from_array(signal),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();
        let signal = self.signal.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(short_ema[i], long_ema[i], signal[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();
        let signal = self.signal.to_array();

        for (i, state) in states.iter_mut().enumerate() {
            state.short_ema = short_ema[i];
            state.long_ema = long_ema[i];
            state.signal = signal[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    multipliers: (
        (Simd<f64, N>, Simd<f64, N>),
        (Simd<f64, N>, Simd<f64, N>),
        (Simd<f64, N>, Simd<f64, N>),
    ),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    //let (mut short_ema, mut long_ema, mut signal) = (state.short_ema, state.long_ema, state.signal);
    let (short_multiplier, long_multiplier, signal_multiplier) = multipliers;
    state.short_ema = calc_ema_simd(value, state.short_ema, short_multiplier);
    state.long_ema = calc_ema_simd(value, state.long_ema, long_multiplier);

    let macd_value = state.short_ema - state.long_ema;
    state.signal = calc_ema_simd(macd_value, state.signal, signal_multiplier);

    (macd_value, state.signal, macd_value - state.signal)
}
