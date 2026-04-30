use crate::indicators::simd_indicators::stddev_simd::{
    calc_simd as stddev_calc_simd, SimdState as SimdStddevState,
};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vidya::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vidya::indicator_by_options;

use crate::indicators::vidya::State;
use std::simd::{Simd, StdFloat};

pub struct SimdState<const N: usize> {
    pub short_state: SimdStddevState<N>,
    pub long_state: SimdStddevState<N>,
    prev_vidya: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &mut [&mut State]) -> Self {
        // Create vectors to collect the references
        let mut short_refs = Vec::with_capacity(N);
        let mut long_refs = Vec::with_capacity(N);

        // Collect references and values
        for state in states.iter_mut() {
            short_refs.push(&mut state.short_state);
            long_refs.push(&mut state.long_state);
        }

        let short_state = SimdStddevState::new(&short_refs);
        let long_state = SimdStddevState::new(&long_refs);
        let prev_vidya = Simd::from_array(std::array::from_fn(|j| states[j].prev_vidya));

        Self {
            short_state,
            long_state,
            prev_vidya,
        }
    }

    pub fn to_states(&self) -> [State; N] {
        let short_states = self.short_state.to_states();
        let long_states = self.long_state.to_states();
        let prev_vidya = self.prev_vidya.to_array();

        // Use into_iter() to consume the arrays and avoid move issues
        let states_vec: Vec<State> = short_states
            .into_iter()
            .zip(long_states.into_iter())
            .zip(prev_vidya.into_iter())
            .map(|((short_state, long_state), prev_vidya)| State {
                short_state,
                long_state,
                prev_vidya,
            })
            .collect();

        // Convert Vec to array
        states_vec
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert states_vec to array"))
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut short_refs = Vec::with_capacity(N);
        let mut long_refs = Vec::with_capacity(N);
        let prev_vidya = self.prev_vidya.to_array();
        // Collect references and values
        for state in states.iter_mut() {
            short_refs.push(&mut state.short_state);
            long_refs.push(&mut state.long_state);
        }
        self.short_state.write_states(&mut short_refs);
        self.long_state.write_states(&mut long_refs);

        for i in 0..N {
            states[i].prev_vidya = prev_vidya[i];
        }
    }
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        value: Simd<f64, N>,
        short_value: Simd<f64, N>,
        long_value: Simd<f64, N>,
        alpha: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        // Compute short-term STDDEV.
        let (multiplier_short, multiplier_long) = multipliers;

        let (sd_short, sma_short) =
            stddev_calc_simd(&mut self.short_state, value, short_value, multiplier_short);

        // Compute long-term STDDEV.
        let (sd_long, sma_long) =
            stddev_calc_simd(&mut self.long_state, value, long_value, multiplier_long);

        let mut k = sd_short / sd_long;
        k *= alpha;

        //self.prev_vidya = (value - self.prev_vidya) * k + self.prev_vidya;
        self.prev_vidya = (value - self.prev_vidya).mul_add(k, self.prev_vidya);
        (self.prev_vidya, sma_short, sma_long, sd_short, sd_long)
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    short_value: Simd<f64, N>,
    long_value: Simd<f64, N>,
    alpha: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> (
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
    Simd<f64, N>,
) {
    state.calc_simd(value, short_value, long_value, alpha, multipliers)
}
