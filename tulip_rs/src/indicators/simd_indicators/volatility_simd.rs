#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::volatility::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::volatility::indicator_by_options;

pub mod imports {
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants,
        stddev_simd::{calc_simd as stddev_calc_simd, SimdState as StddevSimdState},
    };
    pub(crate) use crate::indicators::volatility::State;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::RingBuffer;
    pub(crate) use std::simd::Simd;
}

pub mod assets {
    use super::imports::*;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::{
        SimdBuffer, SimdRingBuffer,
    };
    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub stddev_state: StddevSimdState<N>,
        pub prev_real: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut stddev_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            let mut prev_real = [0.0; N];
            for (i, state) in states.iter_mut().enumerate() {
                stddev_refs.push(&mut state.stddev_state);
                buffer_refs.push(&state.buffer);
                prev_real[i] = state.prev_real;
            }

            let stddev_state = StddevSimdState::new(&mut stddev_refs);
            let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

            Self {
                buffer,
                stddev_state,
                prev_real: Simd::from_array(prev_real),
            }
        }

        pub fn to_states(&self) -> [State; N] {
            let stddev_states = self.stddev_state.to_states();
            let prev_real = self.prev_real.to_array();
            let buffers = self.buffer.to_f64_buffers();
            // Use into_iter() to consume the arrays and avoid move issues
            let states_vec: Vec<State> = buffers
                .into_iter()
                .zip(stddev_states.into_iter())
                .zip(prev_real.iter())
                .map(|((buffer, stddev_state), &prev_real)| State {
                    buffer,
                    stddev_state,
                    prev_real,
                })
                .collect();

            // Convert Vec to array
            states_vec
                .try_into()
                .unwrap_or_else(|_| panic!("Failed to convert states_vec to array"))
        }
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let prev_real = self.prev_real.to_array();
            let mut stddev_refs = Vec::with_capacity(N);

            for (i, (state, buffer)) in states.iter_mut().zip(buffers.into_iter()).enumerate() {
                stddev_refs.push(&mut state.stddev_state);
                state.buffer = buffer;
                state.prev_real = prev_real[i];
            }

            // Finally, update the ADX states
            self.stddev_state.write_states(&mut stddev_refs);
        }
        #[inline(always)]
        pub fn calc_simd(&mut self, real: Simd<f64, N>, multiplier: Simd<f64, N>) -> Simd<f64, N> {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value = self.buffer.push_with_info(value).unwrap();
            let (sd, _) = stddev_calc_simd(&mut self.stddev_state, value, prev_value, multiplier);
            sd * F64Constants::ANNUAL
        }
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            real: Simd<f64, N>,
            multiplier: Simd<f64, N>,
        ) -> Simd<f64, N> {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value = self.buffer.push_with_info_unchecked(value);
            let (sd, _) = stddev_calc_simd(&mut self.stddev_state, value, prev_value, multiplier);
            sd * F64Constants::ANNUAL
        }
    }
}

pub mod options {
    use super::imports::*;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
    pub struct SimdState<const N: usize> {
        pub buffer: Buffer,
        pub stddev_state: StddevSimdState<N>,
        pub prev_real: f64,
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
            let mut stddev_refs = Vec::with_capacity(N);

            for state in states.iter_mut() {
                stddev_refs.push(&mut state.stddev_state);
            }

            let stddev_state = StddevSimdState::new(&mut stddev_refs);

            Self {
                buffer,
                stddev_state,
                prev_real: states[main_buffer].prev_real,
                periods,
            }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let vals: [Vec<f64>; N] =
                std::array::from_fn(|i| self.buffer.to_ordered_by_period(self.periods[i]));

            let prev_real = self.prev_real;
            let mut stddev_refs = Vec::with_capacity(N);

            for (state, vals) in states.iter_mut().zip(vals.into_iter()) {
                stddev_refs.push(&mut state.stddev_state);
                state.buffer = {
                    let len = vals.len();
                    Buffer {
                        vals,
                        index: 0,
                        prev_idx: len - 1,
                        capacity: len,
                        count: len,
                    }
                };
                state.prev_real = prev_real;
            }

            // Finally, update the ADX states
            self.stddev_state.write_states(&mut stddev_refs);
        }
        /*#[inline(always)]
        pub fn calc_simd(&mut self, real: Simd<f64, N>, multiplier: Simd<f64, N>) -> Simd<f64, N> {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value = self.buffer.push_with_info(value).unwrap();
            let (sd, _) = stddev_calc_simd(&mut self.stddev_state, value, prev_value, multiplier);
            sd * F64Constants::ANNUAL
        }*/
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            real: f64,
            multiplier: Simd<f64, N>,
        ) -> Simd<f64, N> {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value = Simd::from_array(
                self.buffer
                    .push_with_info_periods_unchecked(value, self.periods),
            );
            let (sd, _) = stddev_calc_simd(
                &mut self.stddev_state,
                Simd::splat(value),
                prev_value,
                multiplier,
            );
            sd * F64Constants::ANNUAL
        }
    }
}
