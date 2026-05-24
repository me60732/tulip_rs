#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::tema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::tema::indicator_by_options;

use crate::indicators::simd_indicators::{
    dema_simd::{calc_simd as calc_dema_simd, SimdState as DemaSimdState},
    ema_simd::calc_simd as calc_ema_simd,
    simd_types::F64Constants,
};
use crate::indicators::tema::State;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for computing the Triple Exponential Moving Average (TEMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Nested DEMA state, which itself holds EMA1 and EMA2 for each lane.
    pub dema_state: DemaSimdState<N>,
    /// The third-order EMA (`EMA(EMA(EMA(real)))`) for each lane.
    pub ema3: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut dema_state = Vec::with_capacity(N);

        let mut ema3 = [0.0; N];

        for i in 0..N {
            dema_state.push(&states[i].dema_state);
            ema3[i] = states[i].ema3;
        }
        let dema_state = DemaSimdState::new(dema_state.as_slice());

        Self {
            dema_state,
            ema3: Simd::from_array(ema3),
        }
    }
    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let dema_states = self.dema_state.to_states();
        let ema3 = self.ema3.to_array();

        let states: [State; N] =
            std::array::from_fn(|i| State::new(dema_states[i].ema1, dema_states[i].ema2, ema3[i]));

        states
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let dema_states = self.dema_state.to_states();
        let ema3 = self.ema3.to_array();

        for (i, dema_state) in dema_states.into_iter().enumerate() {
            states[i].dema_state = dema_state;
            states[i].ema3 = ema3[i];
        }
    }
}

/// Advances one bar of the TEMA computation for `N` lanes simultaneously.
///
/// Computes `TEMA = 3*EMA1 - 3*EMA2 + EMA3` using fused multiply-add arithmetic for
/// numerical stability. Also returns intermediate DEMA and EMA values as optional outputs.
///
/// # Returns
///
/// `(tema, dema, ema)` — the TEMA, DEMA, and first-order EMA for the current bar.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    multiplier: (Simd<f64, N>, Simd<f64, N>),
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let dema_state = &mut state.dema_state;
    let (dema, ema) = calc_dema_simd(dema_state, value, multiplier);
    state.ema3 = calc_ema_simd(dema_state.ema2, state.ema3, multiplier);

    (
        //F64Constants::THREE * dema_state.ema1 - F64Constants::THREE * dema_state.ema2 + state.ema3,
        dema_state.ema1.mul_add(
            F64Constants::THREE,
            dema_state.ema2.mul_add(-F64Constants::THREE, state.ema3),
        ),
        dema,
        ema,
    )
}
