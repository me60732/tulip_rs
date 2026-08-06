#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::volatility::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::volatility::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
pub mod imports {
    //! Internal imports shared by the [`assets`] and [`options`] SIMD sub-modules
    //! for the Volatility indicator.
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants, stddev_simd::SimdState as StddevSimdState,
    };
    pub(crate) use crate::indicators::volatility::State;
    pub(crate) use crate::types::Warm;
    pub(crate) use std::simd::Simd;
}

pub mod assets {
    //! Per-asset SIMD state and compute for the Volatility indicator.
    use super::imports::*;
    use super::*;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::{
        SimdBuffer, SimdRingBuffer,
    };
    /// SIMD-parallel state for the Volatility indicator, holding `N` lanes of per-asset state.
    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub stddev_state: StddevSimdState<N>,
        pub prev_real: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [(stddev_state: StddevSimdState<N>)],
             scalar: [prev_real],
             buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = Simd<f64, N>;
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Simd<f64, N> {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value = self.buffer.push_with_info(value);
            let (sd, _) = self.stddev_state.calc((value, prev_value));
            sd * F64Constants::ANNUAL
        }
    }
}

pub mod options {
    use super::imports::*;
    use super::*;
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
    /// SIMD-parallel state for the Volatility indicator, holding `N` lanes of per-option state.
    pub struct SimdState<const N: usize> {
        pub buffer: Buffer<Warm>,
        pub stddev_state: StddevSimdState<N>,
        pub prev_real: f64,
        periods: [usize; N],
    }

    impl<const N: usize> SimdState<N> {
        /// Constructs a [`SimdState`] from `N` scalar [`State`] references, one per option-set lane.
        ///
        /// Selects the largest-capacity buffer as the shared ring buffer and initialises the
        /// per-lane stddev state.
        ///
        /// # Arguments
        ///
        /// * `states` - Mutable references to `N` scalar states (one per option set).
        /// * `periods` - Per-lane period values.
        pub fn from_states(states: &mut [&mut State<Warm>], periods: [usize; N]) -> Self {
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

            let stddev_state = StddevSimdState::from_states(&mut stddev_refs);

            Self {
                buffer,
                stddev_state,
                prev_real: states[main_buffer].prev_real,
                periods,
            }
        }

        /// Writes SIMD state back into `N` scalar [`State`] references, one per option-set lane.
        pub fn write_states(&self, states: &mut [&mut State<Warm>]) {
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
                        state: std::marker::PhantomData::<Warm>,
                    }
                };
                state.prev_real = prev_real;
            }

            // Finally, update the ADX states
            self.stddev_state.write_states(&mut stddev_refs);
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = f64;
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
            // Rearranged for better numerical stability when prices are large and close
            let value = (real - self.prev_real) / self.prev_real;
            self.prev_real = real;
            let prev_value =
                Simd::from_array(self.buffer.push_with_info_periods(value, self.periods));
            let (sd, _) = self.stddev_state.calc((Simd::splat(value), prev_value));
            sd * F64Constants::ANNUAL
        }
    }
}
