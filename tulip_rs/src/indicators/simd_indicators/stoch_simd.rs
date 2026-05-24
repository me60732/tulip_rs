#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::stoch::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::stoch::indicator_by_options;
use crate::indicators::{
    simd_indicators::{max_simd::SimdState as MaxSimdState, min_simd::SimdState as MinSimdState},
    stoch::State,
};
use std::simd::{num::SimdFloat, Simd};

pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::Calc as CalcMax, min_simd::assets::Calc as CalcMin,
    };
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };

    /// SIMD-parallel state for computing the Stochastic Oscillator across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        /// Rolling buffer of fast-%K values used to smooth into slow-%K for each lane.
        pub prev_k: SimdBuffer<N>,
        /// Rolling buffer of slow-%K values used to smooth into %D for each lane.
        pub prev_d: SimdBuffer<N>,
        /// Sub-state tracking the rolling minimum of the low prices for the %K window.
        pub min_state: MinSimdState<N>,
        /// Sub-state tracking the rolling maximum of the high prices for the %K window.
        pub max_state: MaxSimdState<N>,
        /// Running sum of fast-%K values within the slow-%K smoothing window for each lane.
        pub k_sum: Simd<f64, N>,
        /// Running sum of slow-%K values within the %D smoothing window for each lane.
        pub d_sum: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Build buffer array directly (immutable references are fine)

            // Build ADX refs using indexing instead of iterator
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut prev_k_refs = Vec::with_capacity(N);
            let mut prev_d_refs = Vec::with_capacity(N);
            let mut k_sum = [0.0; N];
            let mut d_sum = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                prev_k_refs.push(&state.prev_k);
                prev_d_refs.push(&state.prev_d);
                k_sum[i] = state.k_sum;
                d_sum[i] = state.d_sum;
            }

            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);
            let prev_k = SimdBuffer::from_f64_buffers(prev_k_refs);
            let prev_d = SimdBuffer::from_f64_buffers(prev_d_refs);

            Self {
                min_state,
                max_state,
                prev_k,
                prev_d,
                k_sum: Simd::from_array(k_sum),
                d_sum: Simd::from_array(d_sum),
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let k_buffers = self.prev_k.to_f64_buffers();
            let d_buffers = self.prev_d.to_f64_buffers();
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let k_sum = self.k_sum.as_array();
            let d_sum = self.d_sum.as_array();

            for (i, ((state, k_buffer), d_buffer)) in states
                .iter_mut()
                .zip(k_buffers.into_iter())
                .zip(d_buffers.into_iter())
                .enumerate()
            {
                state.prev_k = k_buffer;
                state.prev_d = d_buffer;
                state.k_sum = k_sum[i];
                state.d_sum = d_sum[i];
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
            }

            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
        }

        /// Advances one bar of the Stochastic Oscillator for `N` asset lanes simultaneously.
        ///
        /// Computes fast-%K as `100 * (close - min_low) / (max_high - min_low)` over the
        /// look-back window, then smooths it into slow-%K and further into %D using running sums.
        ///
        /// # Returns
        ///
        /// `(k, d)` — the smoothed slow-%K and %D values for the current bar across all lanes.
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd<const CHUNK_SIZE: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: usize,
            look_back: usize, //k_period - 1
            multipliers: (Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let (k_multiplier, d_multiplier) = multipliers;
            let kfast = {
                let (min, _) = self
                    .min_state
                    .calc_unchecked_simd::<CHUNK_SIZE>(low, i, look_back);
                let (max, _) = self
                    .max_state
                    .calc_unchecked_simd::<CHUNK_SIZE>(high, i, look_back);

                Simd::splat(100.0) * (close - min) / (max - min).simd_max(Simd::splat(f64::EPSILON))
            };

            let old_k = self.prev_k.push_with_info_unchecked(kfast);
            self.k_sum += kfast - old_k;
            let k = self.k_sum * k_multiplier;
            let old_d = self.prev_d.push_with_info_unchecked(k);
            self.d_sum += k - old_d;

            (k, self.d_sum * d_multiplier)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{RingBuffer, UnsyncBuffer};

    /// SIMD-parallel state for computing the Stochastic Oscillator across `N` option sets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to option set `i`.
    pub struct SimdState<const N: usize> {
        /// Rolling buffer of fast-%K values for each option lane.
        pub prev_k: UnsyncBuffer<N, f64>,
        /// Rolling buffer of slow-%K values for each option lane.
        pub prev_d: UnsyncBuffer<N, f64>,
        /// Sub-state tracking the rolling minimum of the low prices for each option's %K window.
        pub min_state: MinSimdState<N>,
        /// Sub-state tracking the rolling maximum of the high prices for each option's %K window.
        pub max_state: MaxSimdState<N>,
        /// Running sum of fast-%K values within the slow-%K smoothing window for each option lane.
        pub k_sum: Simd<f64, N>,
        /// Running sum of slow-%K values within the %D smoothing window for each option lane.
        pub d_sum: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Build buffer array directly (immutable references are fine)

            // Build ADX refs using indexing instead of iterator
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut prev_k_refs = Vec::with_capacity(N);
            let mut prev_d_refs = Vec::with_capacity(N);
            let mut k_sum = [0.0; N];
            let mut d_sum = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                prev_k_refs.push(&state.prev_k);
                prev_d_refs.push(&state.prev_d);
                k_sum[i] = state.k_sum;
                d_sum[i] = state.d_sum;
            }

            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);
            let prev_k = UnsyncBuffer::from_buffers(prev_k_refs);
            let prev_d = UnsyncBuffer::from_buffers(prev_d_refs);

            Self {
                min_state,
                max_state,
                prev_k,
                prev_d,
                k_sum: Simd::from_array(k_sum),
                d_sum: Simd::from_array(d_sum),
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let k_buffers = self.prev_k.to_f64_buffers();
            let d_buffers = self.prev_d.to_f64_buffers();

            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let k_sum = self.k_sum.as_array();
            let d_sum = self.d_sum.as_array();

            for (i, ((state, k_buffer), d_buffer)) in states
                .iter_mut()
                .zip(k_buffers.into_iter())
                .zip(d_buffers.into_iter())
                .enumerate()
            {
                state.prev_k = k_buffer;
                state.prev_d = d_buffer;
                state.k_sum = k_sum[i];
                state.d_sum = d_sum[i];
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
            }

            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
        }

        /// Advances one bar of the Stochastic Oscillator for `N` option lanes simultaneously.
        ///
        /// Each lane uses its own look-back window sizes (lane-specific `k_period`). Computes
        /// fast-%K, smooths it into slow-%K, then further into %D.
        ///
        /// # Returns
        ///
        /// `(k, d)` — the smoothed slow-%K and %D values for the current bar across all option lanes.
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: Simd<usize, N>,
            look_back: Simd<usize, N>, //k_period - 1
            multipliers: (Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let (k_multiplier, d_multiplier) = multipliers;
            let kfast = {
                let (min, _) = self.min_state.calc_unchecked_simd(low, i, look_back);
                let (max, _) = self.max_state.calc_unchecked_simd(high, i, look_back);

                Simd::splat(100.0) * (close - min) / (max - min).simd_max(Simd::splat(f64::EPSILON))
            };

            let old_k = self.prev_k.push_with_info_unchecked(kfast);
            self.k_sum += kfast - old_k;
            let k = self.k_sum * k_multiplier;
            let old_d = self.prev_d.push_with_info_unchecked(k);
            self.d_sum += k - old_d;

            (k, self.d_sum * d_multiplier)
        }
    }
}
