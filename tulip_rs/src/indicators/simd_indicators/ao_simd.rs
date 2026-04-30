use crate::indicators::ao::{State, SHORT_PERIOD};

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ao::indicator_by_assets;

use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, sma_simd::calc_simd as calc_sma_simd,
};
use crate::ring_buffer::single_buffer::generic_buffer::{RingBuffer, SimdBuffer, SimdRingBuffer};
use std::simd::Simd;
pub struct SimdState<const N: usize> {
    buffer: SimdBuffer<N>,
    pub short_sum: Simd<f64, N>,
    pub long_sum: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    pub fn new(states: &mut [&mut State]) -> Self {
        debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");
        let mut short_sum = [0.0; N];
        let mut long_sum = [0.0; N];

        let mut buffer_refs = Vec::with_capacity(N);
        for (i, state) in states.iter_mut().enumerate() {
            short_sum[i] = state.short_sum;
            long_sum[i] = state.long_sum;
            buffer_refs.push(&state.buffer)
        }

        let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

        Self {
            buffer,
            short_sum: Simd::from_array(short_sum),
            long_sum: Simd::from_array(long_sum),
        }
    }

    pub fn write_states(&self, states: &mut [&mut State]) {
        // First, handle the buffer updates
        let buffers = self.buffer.to_f64_buffers();
        let short_sum = self.short_sum.as_array();
        let long_sum = self.long_sum.as_array();

        for (i, buffer) in buffers.into_iter().enumerate() {
            states[i].buffer = buffer;
            states[i].short_sum = short_sum[i];
            states[i].long_sum = long_sum[i];
        }
    }

    #[inline(always)]
    pub unsafe fn calc_unchecked_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (short_multiplier, long_multiplier) = multipliers;

        let med_price = calc_medprice_simd(high, low);

        let long_sma = calc_sma_simd(
            &mut self.long_sum,
            med_price,
            self.buffer.push_with_info_unchecked(med_price),
            long_multiplier,
        );
        let short_sma = calc_sma_simd(
            &mut self.short_sum,
            med_price,
            self.buffer.get_by_period(SHORT_PERIOD),
            short_multiplier,
        );

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (short_multiplier, long_multiplier) = multipliers;

        let med_price = calc_medprice_simd(high, low);

        let long_sma = if let Some(prev) = self.buffer.push_with_info(med_price) {
            calc_sma_simd(&mut self.long_sum, med_price, prev, long_multiplier)
        } else {
            Simd::splat(0.0)
        };

        let short_sma = calc_sma_simd(
            &mut self.short_sum,
            med_price,
            self.buffer.get_by_period(SHORT_PERIOD),
            short_multiplier,
        );

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
}
