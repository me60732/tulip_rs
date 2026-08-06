use crate::indicators::ao::{IndicatorState as State, SHORT_PERIOD};

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ao::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, sma_simd::SimdState as SmaSimdState,
};
use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};
use std::simd::Simd;
/// SIMD-parallel state for computing the Awesome Oscillator (AO) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    buffer: SimdBuffer<N>,
    pub short_sma_state: SmaSimdState<N>,
    pub long_sma_state: SmaSimdState<N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;

    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    fn from_states(states: &mut [&mut State]) -> Self {
        debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");
        let mut short_sum = [0.0; N];
        let mut long_sum = [0.0; N];
        let mut short_multipliers = [0.0; N];
        let mut long_multipliers = [0.0; N];

        let mut buffer_refs = Vec::with_capacity(N);
        for (i, state) in states.iter_mut().enumerate() {
            let [short, long] = state.sma_state.sum.to_array();
            short_sum[i] = short;
            long_sum[i] = long;
            short_multipliers[i] = state.sma_state.multiplier[0];
            long_multipliers[i] = state.sma_state.multiplier[1];
            buffer_refs.push(&state.buffer)
        }

        let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

        Self {
            buffer,
            short_sma_state: SmaSimdState::new(
                Simd::from_array(short_sum),
                Simd::from_array(short_multipliers),
            ),
            long_sma_state: SmaSimdState::new(
                Simd::from_array(long_sum),
                Simd::from_array(long_multipliers),
            ),
        }
    }

    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    fn write_states(&self, states: &mut [&mut State]) {
        // First, handle the buffer updates
        let buffers = self.buffer.to_f64_buffers();
        let short_sum = self.short_sma_state.sum.as_array();
        let long_sum = self.long_sma_state.sum.as_array();

        for (i, buffer) in buffers.into_iter().enumerate() {
            let [short, long] = states[i].sma_state.sum.as_mut_array();
            states[i].buffer = buffer;
            *short = short_sum[i];
            *long = long_sum[i];
        }
    }
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    /// Advances the AO by one bar for `N` assets simultaneously.
    ///
    /// Computes the median price, pushes it into the ring buffer, then updates both the short-
    /// and long-period SMAs. Returns `(ao, short_sma, long_sma, medprice)`.
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Self::Outputs {
        let med_price = calc_medprice_simd(high, low);

        let long_sma = self
            .long_sma_state
            .calc((med_price, self.buffer.push_with_info(med_price)));
        let short_sma = self
            .short_sma_state
            .calc((med_price, self.buffer.get_by_period(SHORT_PERIOD)));

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
}
