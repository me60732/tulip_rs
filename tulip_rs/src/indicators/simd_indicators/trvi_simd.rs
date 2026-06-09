#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trvi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trvi::indicator_by_options;

pub(crate) mod import {
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::calc_simd as ema_calc_simd, simd_types::F64Constants,
    };
    pub(crate) use crate::indicators::trvi::State;
    pub(crate) use std::simd::{Select, Simd};
}

pub mod assets {
    pub(crate) use super::import::*;
    use crate::indicators::simd_indicators::tr_simd::calc_simd as tr_calc_simd;
    /// SIMD state alias for the TRVI assets path — the state is a [`SimdBuffer`] of EMA values,
    /// one per asset lane, sized to the indicator's lookback period.
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::SimdBuffer;
    use crate::ring_buffer::single_buffer::generic_buffer::{RingBuffer, SimdRingBuffer};
    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub prev_close: Simd<f64, N>,
    }
    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            let mut buffer_refs = Vec::with_capacity(N);
            let mut prev_close = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                buffer_refs.push(&state.buffer);
                prev_close[i] = state.prev_close;
            }
            let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

            Self {
                buffer,
                prev_close: Simd::from_array(prev_close),
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in
        /// place, avoiding allocation compared to a `to_states` conversion.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let prev_close = self.prev_close.as_array();
            for (i, (buffer, state)) in buffers.into_iter().zip(states.iter_mut()).enumerate() {
                state.buffer = buffer;
                state.prev_close = prev_close[i];
            }
        }

        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            close: Simd<f64, N>,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let prev_ema = self.buffer.back_unchecked();
            let old_ema = self.buffer.front_unchecked();
            let tr = tr_calc_simd(high, low, self.prev_close);
            let ema = ema_calc_simd(tr, prev_ema, multiplier);
            self.buffer.push_unchecked(ema);
            self.prev_close = close;

            ((ema - old_ema) / old_ema * F64Constants::HUNDRED, tr, ema)
        }
    }
}

pub mod options {
    pub(crate) use super::import::*;
    use crate::indicators::tr::calc as calc_tr;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::RingBuffer;
    /// SIMD state alias for the TRVI options path — per-lane ring buffers with potentially
    /// different periods stored in an `UnsyncBuffer`.
    pub(crate) use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;
    pub struct SimdState<const N: usize> {
        pub buffer: UnsyncBuffer<N, f64>,
        pub prev_close: f64,
    }
    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");
            let prev_close = states[0].prev_close;
            let mut buffer_refs = Vec::with_capacity(N);

            for state in states.iter_mut() {
                buffer_refs.push(&state.buffer);
            }
            let buffer = UnsyncBuffer::from_buffers(buffer_refs);
            Self { buffer, prev_close }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in
        /// place, avoiding allocation compared to a `to_states` conversion.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let prev_close = self.prev_close;
            for (state, buffer) in states.iter_mut().zip(buffers.into_iter()) {
                state.buffer = buffer;
                state.prev_close = prev_close;
            }
        }

        #[inline]
        pub fn calc_simd(
            &mut self,
            high: f64,
            low: f64,
            close: f64,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let tr = Simd::splat(calc_tr(high, low, self.prev_close));
            let prev_ema = self.buffer.back_unchecked();
            let (old_ema, old_ema_mask) = self.buffer.front();
            let ema = ema_calc_simd(tr, prev_ema, multiplier);
            self.buffer.push(ema);
            self.prev_close = close;

            let trvi = old_ema_mask.select(
                (ema - old_ema) / old_ema * F64Constants::HUNDRED,
                F64Constants::ZERO,
            );
            (trvi, tr, ema)
        }

        /// Advances the TRVI by one bar for `N` option lanes simultaneously (unchecked variant).
        ///
        /// # Safety
        ///
        /// The caller must guarantee all per-lane ring buffers are fully warmed up.
        #[inline(always)]
        pub(crate) unsafe fn calc_unchecked_simd(
            &mut self,
            high: f64,
            low: f64,
            close: f64,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let tr = Simd::splat(calc_tr(high, low, self.prev_close));

            let prev_ema = self.buffer.back_unchecked();
            let old_ema = self.buffer.front_unchecked();

            let ema = ema_calc_simd(tr, prev_ema, multiplier);
            self.buffer.push_unchecked(ema);
            self.prev_close = close;

            ((ema - old_ema) / old_ema * F64Constants::HUNDRED, tr, ema)
        }
    }
}
