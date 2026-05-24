use crate::indicators::ppo::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ppo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ppo::indicator_by_options;

use crate::indicators::simd_indicators::{
    ema_simd::calc_simd as calc_ema_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, *};

/// SIMD-parallel state for the Percentage Price Oscillator (PPO) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub short_ema: Simd<f64, N>,
    pub long_ema: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    pub fn new(states: &[&mut State]) -> Self {
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];

        for i in 0..N {
            short_ema[i] = states[i].short_ema;
            long_ema[i] = states[i].long_ema;
        }
        Self {
            short_ema: Simd::from_array(short_ema),
            long_ema: Simd::from_array(long_ema),
        }
    }
    /*pub fn to_states(&self) -> [State; N] {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(short_ema[i], long_ema[i]));

        states
    }*/
    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        for i in 0..N {
            states[i].short_ema = short_ema[i];
            states[i].long_ema = long_ema[i];
        }
    }
    /// Computes one bar of the Percentage Price Oscillator (PPO) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Updates the short-term and long-term EMAs and returns
    /// `(short_ema - long_ema) * 100 / long_ema`.
    ///
    /// # Arguments
    ///
    /// * `real` - Current prices for this bar.
    /// * `multipliers` - Tuple of `(short_multiplier, long_multiplier)` EMA multiplier pairs.
    ///
    /// # Returns
    ///
    /// PPO values for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        real: Simd<f64, N>,
        multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
    ) -> Simd<f64, N> {
        let (short_multiplier, long_multiplier) = multipliers;

        self.short_ema = calc_ema_simd(real, self.short_ema, short_multiplier);
        self.long_ema = calc_ema_simd(real, self.long_ema, long_multiplier);

        let long_ema_safe = self.long_ema.simd_max(F64Constants::EPSILON);
        (self.short_ema - self.long_ema) * F64Constants::HUNDRED / long_ema_safe
    }
}
