#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adxr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adxr::indicator_by_options;

mod imports {
    pub(crate) use crate::indicators::adxr::State;
    pub(crate) use crate::indicators::simd_indicators::{
        adx_simd::{calc_simd as adx_calc_simd, SimdState as AdxSimdState},
        simd_types::F64Constants,
    };
    pub(crate) use std::simd::{Select, Simd};
}

pub mod assets {
    use super::imports::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };

    pub struct SimdState<const N: usize> {
        pub adx_state: AdxSimdState<N>,
        pub buffer: SimdBuffer<N>,
    }
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Build buffer array directly (immutable references are fine)

            // Build ADX refs using indexing instead of iterator
            let mut adx_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            for state in states.iter_mut() {
                adx_refs.push(&mut state.adx_state);
                buffer_refs.push(&state.buffer)
            }

            let adx_state = AdxSimdState::new(&mut adx_refs);
            let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

            Self { adx_state, buffer }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            for (i, buffer) in buffers.into_iter().enumerate() {
                states[i].buffer = buffer;
            }

            // Now collect ADX references using iter_mut() instead of indexing
            let mut adx_refs = Vec::with_capacity(N);
            for state in states.iter_mut() {
                adx_refs.push(&mut state.adx_state);
            }

            // Finally, update the ADX states
            self.adx_state.write_states(&mut adx_refs);
        }
    }

    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multiplier);

        let prev_adx = state.buffer.push_with_info(adx);
        let mut adxr = F64Constants::ZERO;
        if let Some(pa) = prev_adx {
            adxr = F64Constants::HALF * (adx + pa);
        }

        (adxr, adx, dx, atr, tr)
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multiplier);
        let adxr = F64Constants::HALF * (adx + state.buffer.push_with_info_unchecked(adx));

        (adxr, adx, dx, atr, tr)
    }
}

pub mod options {
    use super::imports::*;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{RingBuffer, UnsyncBuffer};

    pub struct SimdState<const N: usize> {
        pub adx_state: AdxSimdState<N>,
        pub buffer: UnsyncBuffer<N, f64>,
    }
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Build buffer array directly (immutable references are fine)

            // Build ADX refs using indexing instead of iterator
            let mut adx_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            for state in states.iter_mut() {
                adx_refs.push(&mut state.adx_state);
                buffer_refs.push(&state.buffer)
            }

            let adx_state = AdxSimdState::new(&mut adx_refs);
            let buffer = UnsyncBuffer::from_buffers(buffer_refs);

            Self { adx_state, buffer }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let mut adx_refs = Vec::with_capacity(N);
            for (buffer, state) in buffers.into_iter().zip(states.iter_mut()) {
                state.buffer = buffer;
                adx_refs.push(&mut state.adx_state);
            }

            // Finally, update the ADX states
            self.adx_state.write_states(&mut adx_refs);
        }
    }

    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multiplier);

        let (prev_adx, mask) = state.buffer.push_with_info(adx);

        let adxr = mask.select(F64Constants::HALF * (adx + prev_adx), F64Constants::ZERO);

        (adxr, adx, dx, atr, tr)
    }
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multiplier);
        let adxr = F64Constants::HALF * (adx + state.buffer.push_with_info_unchecked(adx));

        (adxr, adx, dx, atr, tr)
    }
}
