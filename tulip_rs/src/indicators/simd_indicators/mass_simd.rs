#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::mass::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::mass::indicator_by_options;

pub mod imports {
    pub(crate) use crate::indicators::mass::State;
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::calc_simd as ema_calc_simd, simd_types::F64Constants,
    };
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}

pub mod asset {
    use super::imports::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };

    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub sum: Simd<f64, N>,
        pub ema: Simd<f64, N>,
        pub ema_signal: Simd<f64, N>,
    }
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut buffer_refs = Vec::with_capacity(N);
            let mut sum = [0.0; N];
            let mut ema = [0.0; N];
            let mut ema_signal = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                buffer_refs.push(&state.buffer);
                sum[i] = state.sum;
                ema[i] = state.ema;
                ema_signal[i] = state.ema_signal;
            }

            let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

            Self {
                buffer,
                sum: Simd::from_array(sum),
                ema: Simd::from_array(ema),
                ema_signal: Simd::from_array(ema_signal),
            }
        }
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let sum = self.sum.to_array();
            let ema = self.ema.to_array();
            let ema_signal = self.ema_signal.to_array();

            for (i, (buffer, state)) in buffers.into_iter().zip(states.iter_mut()).enumerate() {
                state.buffer = buffer;
                state.sum = sum[i];
                state.ema = ema[i];
                state.ema_signal = ema_signal[i];
            }
        }
        #[inline(always)]
        pub fn calc_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N> {
            let mass;
            (mass, self.ema, self.ema_signal) =
                calc_mass((self.ema, self.ema_signal), high, low, multiplier);
            if let Some(old) = self.buffer.push_with_info(mass) {
                self.sum -= old
            }
            self.sum += mass;
            self.sum
        }
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N> {
            let mass;
            (mass, self.ema, self.ema_signal) =
                calc_mass((self.ema, self.ema_signal), high, low, multiplier);
            self.sum += mass - self.buffer.push_with_info_unchecked(mass);
            self.sum
        }
    }
    #[inline(always)]
    pub(crate) fn calc_mass<const N: usize>(
        emas: (Simd<f64, N>, Simd<f64, N>),
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        multiplier: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
        let (mut ema, mut ema_signal) = emas;
        ema = ema_calc_simd(hl_diff, ema, multiplier);
        ema_signal = ema_calc_simd(ema, ema_signal, multiplier);
        (
            (ema / ema_signal).simd_max(F64Constants::ZERO),
            ema,
            ema_signal,
        )
    }
}

pub mod option {
    use super::imports::*;
    use crate::indicators::ema::calc as ema_calc;
    use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};

    pub struct SimdState<const N: usize> {
        pub buffer: Buffer,
        pub sum: Simd<f64, N>,
        pub ema: f64,
        pub ema_signal: f64,
        periods: [usize; N],
    }
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State], periods: [usize; N]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut main_buffer = 0;
            for i in 1..N {
                if states[main_buffer].buffer.capacity < states[i].buffer.capacity {
                    main_buffer = i;
                }
            }
            let buffer = states[main_buffer].buffer.clone();
            let mut sum = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                sum[i] = state.sum;
            }

            Self {
                buffer,
                sum: Simd::from_array(sum),
                ema: states[0].ema,
                ema_signal: states[0].ema_signal,
                periods,
            }
        }
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates

            let vals: [Vec<f64>; N] =
                std::array::from_fn(|i| self.buffer.to_ordered_by_period(self.periods[i]));
            let sum = self.sum.to_array();

            for (i, (val, state)) in vals.into_iter().zip(states.iter_mut()).enumerate() {
                state.buffer = Buffer {
                    index: 0,
                    prev_idx: val.len() - 1,
                    capacity: val.len(),
                    count: val.len(),
                    vals: val,
                };
                state.sum = sum[i];
                state.ema = self.ema;
                state.ema_signal = self.ema_signal;
            }
        }

        #[inline(always)]
        pub(crate) unsafe fn calc_unchecked(
            &mut self,
            high: f64,
            low: f64,
            multiplier: (f64, f64),
        ) -> Simd<f64, N> {
            let hl_diff = (high - low).max(f64::EPSILON);
            let (mut ema, mut ema_signal) = (self.ema, self.ema_signal);
            ema = ema_calc(&hl_diff, ema, multiplier);
            ema_signal = ema_calc(&ema, ema_signal, multiplier);
            let mass = (ema / ema_signal).max(0.0);
            self.sum += Simd::splat(mass)
                - Simd::from_array(
                    self.buffer
                        .push_with_info_periods_unchecked(mass, self.periods),
                );

            (self.ema, self.ema_signal) = (ema, ema_signal);
            self.sum
        }
    }
}
