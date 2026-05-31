#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::stochrsi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::stochrsi::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as MaxSimdState, min_simd::SimdState as MinSimdState,
    rsi_simd::SimdState as RsiSimdState,
};
use crate::indicators::stochrsi::State;
use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer as SingleMirrorBuffer;
use std::f64;
use std::simd::{cmp::SimdPartialOrd, Select, Simd};

pub mod assets {
    use super::*;
    use crate::ring_buffer::multi_buffer::mirror_buffer::{MinMaxBuffer, MirrorBuffer};
    use crate::ring_buffer::multi_buffer::multi_buffer::MultiBuffer;

    /// SIMD-parallel state for computing the Stochastic RSI across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        /// Rolling buffer of RSI values for each lane, used to find the min/max RSI over the period.
        pub buffer: MultiBuffer<N>,
        /// Sub-state tracking the rolling minimum of RSI values for each lane.
        pub min_state: MinSimdState<N>,
        /// Sub-state tracking the rolling maximum of RSI values for each lane.
        pub max_state: MaxSimdState<N>,
        /// Sub-state tracking the underlying RSI computation for each lane.
        pub rsi_state: RsiSimdState<N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut rsi_refs = Vec::with_capacity(N);
            let mut buffer_slices = Vec::with_capacity(N);
            let capacity = states[0].buffer.capacity;

            for state in states.iter_mut() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                rsi_refs.push(&mut state.rsi_state);
                buffer_slices.push(state.buffer.get_slice());
            }

            let buffer_refs: [&[f64]; N] =
                buffer_slices.try_into().unwrap_or_else(|v: Vec<&[f64]>| {
                    panic!("Expected {} buffer slices, got {}", N, v.len())
                });

            let buffer = MultiBuffer::from_slice(buffer_refs, capacity);
            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);
            let rsi_state = RsiSimdState::new(&mut rsi_refs);

            Self {
                buffer,
                min_state,
                max_state,
                rsi_state,
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
        pub fn write_states(&self, states: &mut [&mut State]) {
            let mut max_refs = Vec::with_capacity(N);
            let mut min_refs = Vec::with_capacity(N);
            let mut rsi_refs = Vec::with_capacity(N);

            let buffers = self.buffer.to_single_buffers();
            // Collect references and values
            // Use zip to pair states with buffers
            for (state, buffer) in states.iter_mut().zip(buffers.into_iter()) {
                max_refs.push(&mut state.max_state);
                min_refs.push(&mut state.min_state);
                rsi_refs.push(&mut state.rsi_state);
                state.buffer = buffer;
            }
            self.rsi_state.write_states(&mut rsi_refs);
            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
        }

        /// Advances one bar of the Stochastic RSI computation for `N` asset lanes simultaneously.
        ///
        /// Computes RSI for the new bar, appends it to the rolling RSI buffer, then applies
        /// the stochastic formula `100 * (rsi - min_rsi) / (max_rsi - min_rsi)` over the window.
        /// Returns 0 when the RSI range is below [`f64::EPSILON`].
        ///
        /// # Returns
        ///
        /// `(stochrsi, rsi)` — the Stochastic RSI value and the underlying RSI value for each lane.
        #[inline(always)]
        pub fn calc_simd<const CHUNK_SIZE: usize>(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>),
            period: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let rsi = self.rsi_state.calc_simd(real, multipliers);
            self.buffer.push(rsi.to_array());

            let (min, _) = self
                .buffer
                .min::<CHUNK_SIZE>(&mut self.min_state, rsi, period);
            let (max, _) = self
                .buffer
                .max::<CHUNK_SIZE>(&mut self.max_state, rsi, period);

            let kdif = max - min;

            let kfast = kdif
                .simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (rsi - min) / kdif);

            (kfast, rsi)
        }
    }
}

pub mod options {
    use super::*;
    use crate::ring_buffer::{
        unsync_multi_buffer::{
            mirror_buffer::MinMaxBuffer,
            multi_buffer::{MirrorBuffer, UnsyncBuffer},
        },
        //single_buffer::mirror_buffer::MirrorBuffer as SingleMirrorBuffer,
    };

    /// SIMD-parallel state for computing the Stochastic RSI across `N` option sets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to option set `i`.
    pub struct SimdState<const N: usize> {
        /// Rolling buffer of RSI values for each option lane.
        pub buffer: UnsyncBuffer<N, f64>,
        /// Sub-state tracking the rolling minimum of RSI values for each option lane.
        pub min_state: MinSimdState<N>,
        /// Sub-state tracking the rolling maximum of RSI values for each option lane.
        pub max_state: MaxSimdState<N>,
        /// Sub-state tracking the underlying RSI computation for each option lane.
        pub rsi_state: RsiSimdState<N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut rsi_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);

            // Collect references and values
            for state in states.iter_mut() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                rsi_refs.push(&mut state.rsi_state);
                buffer_refs.push(&state.buffer);
            }

            let buffer = UnsyncBuffer::from_buffers(buffer_refs);
            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);
            let rsi_state = RsiSimdState::new(&mut rsi_refs);

            Self {
                buffer,
                min_state,
                max_state,
                rsi_state,
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
        pub fn write_states(&self, states: &mut [&mut State]) {
            let mut max_refs = Vec::with_capacity(N);
            let mut min_refs = Vec::with_capacity(N);
            let mut rsi_refs = Vec::with_capacity(N);
            let buffers = self.buffer.to_f64_buffers();
            // Collect references and values
            // Use zip to pair states with buffers
            for (state, buffer) in states.iter_mut().zip(buffers.into_iter()) {
                max_refs.push(&mut state.max_state);
                min_refs.push(&mut state.min_state);
                rsi_refs.push(&mut state.rsi_state);
                state.buffer = buffer;
            }

            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
            self.rsi_state.write_states(&mut rsi_refs);
        }

        /// Advances one bar of the Stochastic RSI computation for `N` option lanes simultaneously.
        ///
        /// Each lane uses its own period. Computes RSI, then applies the stochastic formula
        /// over each lane's own rolling window size.
        ///
        /// # Returns
        ///
        /// `(stochrsi, rsi)` — the Stochastic RSI and underlying RSI for each option lane.
        #[inline(always)]
        pub fn calc_simd(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>),
            period: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let rsi = self.rsi_state.calc_simd(real, multipliers);
            self.buffer.push(rsi);

            let (min, _) = self.buffer.min(&mut self.min_state, rsi, period);
            let (max, _) = self.buffer.max(&mut self.max_state, rsi, period);

            let kdif = max - min;

            let kfast = kdif
                .simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (rsi - min) / kdif);

            (kfast, rsi)
        }

        /// Advances one bar of the Stochastic RSI assuming the buffer is already full.
        ///
        /// # Safety
        ///
        /// Caller must guarantee the internal RSI buffer has been fully populated before calling.
        #[inline(always)]
        pub unsafe fn calc_simd_unchecked(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>),
            period: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let rsi = self.rsi_state.calc_simd(real, multipliers);
            self.buffer.push_unchecked(rsi);

            let (min, _) = self.buffer.min(&mut self.min_state, rsi, period);
            let (max, _) = self.buffer.max(&mut self.max_state, rsi, period);

            let kdif = max - min;

            let kfast = kdif
                .simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (rsi - min) / kdif);

            (kfast, rsi)
        }
    }
}
