pub use crate::indicators::apo::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::apo::indicator_by_assets;
use crate::indicators::simd_indicators::ema_simd::calc_simd as calc_ema_simd;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::apo::indicator_by_options;

use std::simd::Simd;

/// Advances the Absolute Price Oscillator (APO) by one bar for `N` assets simultaneously.
///
/// Updates the short- and long-period EMAs, then returns `short_ema - long_ema`.
///
/// # Arguments
///
/// * `state` - Mutable SIMD state holding per-asset short and long EMAs.
/// * `real` - Input price values for the current bar.
/// * `multipliers` - Tuple of `(short_multiplier, long_multiplier)` EMA smoothing factors.
///
/// # Returns
///
/// APO values (`short_ema - long_ema`) for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    real: Simd<f64, N>,
    multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
) -> Simd<f64, N> {
    let (short_multiplier, long_multiplier) = multipliers;
    state.short_ema = calc_ema_simd(real, state.short_ema, short_multiplier);
    state.long_ema = calc_ema_simd(real, state.long_ema, long_multiplier);

    state.short_ema - state.long_ema
}

/// SIMD-parallel state for computing the Absolute Price Oscillator (APO) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Short-period EMA of the price series for each asset.
    pub short_ema: Simd<f64, N>,
    /// Long-period EMA of the price series for each asset.
    pub long_ema: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
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
    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(short_ema[i], long_ema[i]));

        states
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to [`to_states`].
    pub fn write_states(&self, states: &mut [&mut State]) {
        let short_ema = self.short_ema.to_array();
        let long_ema = self.long_ema.to_array();

        for i in 0..N {
            states[i].short_ema = short_ema[i];
            states[i].long_ema = long_ema[i];
        }
    }
}
