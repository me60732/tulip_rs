#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cvi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cvi::indicator_by_options;

pub(crate) mod import {
    pub(crate) use crate::indicators::cvi::State;
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::calc_simd as ema_calc_simd, simd_types::F64Constants,
    };
    pub(crate) use std::simd::{num::SimdFloat, Select, Simd};
    pub trait SimdBufferExt {
        fn new(states: &mut [&mut State]) -> Self;
        fn write_states(&self, states: &mut [&mut State]);
    }
}

pub mod assets {
    pub(crate) use super::import::*;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::SimdBuffer as SimdState;
    use crate::ring_buffer::single_buffer::generic_buffer::{RingBuffer, SimdRingBuffer};

    impl<const N: usize> SimdBufferExt for SimdState<N> {
        fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let buffers: Vec<&State> = states.iter().map(|state| *state as &State).collect();
            SimdState::from_f64_buffers(buffers)
        }

        fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.to_f64_buffers();
            for (i, buffer) in buffers.into_iter().enumerate() {
                *states[i] = buffer;
            }
        }
    }

    #[inline]
    pub fn calc_simd<const N: usize>(
        buffer: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let prev_ema = buffer.back().unwrap();
        let old_ema = buffer.front().unwrap();
        let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
        let ema = ema_calc_simd(hl_diff, prev_ema, multiplier);
        buffer.push(ema);
        if old_ema.abs() < F64Constants::EPSILON {
            F64Constants::ZERO
        } else {
            (ema - old_ema) / old_ema * F64Constants::HUNDRED
        }
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked_simd<const N: usize>(
        buffer: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let prev_ema = buffer.back_unchecked();
        let old_ema = buffer.front_unchecked();
        let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
        let ema = ema_calc_simd(hl_diff, prev_ema, multiplier);
        buffer.push_unchecked(ema);

        (ema - old_ema) / old_ema * F64Constants::HUNDRED
    }
}

pub mod options {
    pub(crate) use super::import::*;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::RingBuffer;
    pub(crate) use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer as SimdState;

    impl<const N: usize> SimdBufferExt for SimdState<N, f64> {
        fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut buffer_refs = Vec::with_capacity(N);
            for state in states.iter() {
                buffer_refs.push(&**state)
            }
            SimdState::from_buffers(buffer_refs)
        }

        fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.to_f64_buffers();
            for (i, buffer) in buffers.into_iter().enumerate() {
                *states[i] = buffer;
            }
        }
    }

    #[inline]
    pub fn calc_simd<const N: usize>(
        buffer: &mut SimdState<N, f64>,
        high: f64,
        low: f64,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let hl_diff = Simd::splat((high - low).max(f64::EPSILON));
        let prev_ema = buffer.back_unchecked();
        let (old_ema, old_ema_mask) = buffer.front();
        let ema = ema_calc_simd(hl_diff, prev_ema, multiplier);
        buffer.push(ema);

        old_ema_mask.select(
            (ema - old_ema) / old_ema * F64Constants::HUNDRED,
            F64Constants::ZERO,
        )
    }
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked_simd<const N: usize>(
        buffer: &mut SimdState<N, f64>,
        high: f64,
        low: f64,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let hl_diff = Simd::splat((high - low).max(f64::EPSILON));

        let prev_ema = buffer.back_unchecked();
        let old_ema = buffer.front_unchecked();

        let ema = ema_calc_simd(hl_diff, prev_ema, multiplier);
        buffer.push_unchecked(ema);

        (ema - old_ema) / old_ema * F64Constants::HUNDRED
    }
}
