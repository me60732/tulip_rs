use crate::indicators::fosc::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::fosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::fosc::indicator_by_options;

use crate::indicators::simd_indicators::{
    simd_types::F64Constants,
    tsf_simd::{calc_simd as tsf_calc_simd, SimdState as SimdLinregState},
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Forecast Oscillator (FOSC) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Underlying linear-regression / TSF SIMD state carrying the per-asset sum accumulators.
    linreg_state: SimdLinregState<N>,
    /// Most recent Time Series Forecast (TSF) value per asset lane, used on the next bar to compute FOSC.
    tsf: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut linreg_state = Vec::with_capacity(N);

        let mut tsf = [0.0; N];

        for i in 0..N {
            linreg_state.push(&states[i].linreg_state);
            tsf[i] = states[i].tsf;
        }
        let linreg_state = SimdLinregState::new(linreg_state.as_slice());

        Self {
            linreg_state,
            tsf: Simd::from_array(tsf),
        }
    }
    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let linreg_states = self.linreg_state.to_states();
        let tsf = self.tsf.to_array();

        let states: [State; N] = std::array::from_fn(|i| {
            State::new(
                tsf[i],
                linreg_states[i].sum_x,
                linreg_states[i].sum_y,
                linreg_states[i].sum_xy,
                linreg_states[i].per,
            )
        });

        states
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let linreg_states = self.linreg_state.to_states();
        let tsf = self.tsf.to_array();

        for (i, linreg_state) in linreg_states.into_iter().enumerate() {
            states[i].linreg_state = linreg_state;
            states[i].tsf = tsf[i];
        }
    }
}

/// Computes one FOSC step across `N` asset lanes using SIMD parallelism.
///
/// FOSC measures the percentage deviation of the current price from the Time Series
/// Forecast: `fosc = 100 * (value - tsf_prev) / value`. It then advances the
/// underlying linear-regression / TSF state so that `tsf_prev` is ready for the
/// next bar.
///
/// Returns `(fosc, tsf, linreg, slope, intercept)` for all `N` lanes simultaneously.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    prev_value: Simd<f64, N>,
    value: Simd<f64, N>,
    period: Simd<f64, N>,
) -> (
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
) {
    let fosc = F64Constants::HUNDRED * (value - state.tsf) / value; //.max(f64::EPSILON);

    let (tsf, linreg, slope, intercept) =
        tsf_calc_simd(&mut state.linreg_state, prev_value, value, period);
    state.tsf = tsf;
    (fosc, tsf, linreg, slope, intercept)
}
