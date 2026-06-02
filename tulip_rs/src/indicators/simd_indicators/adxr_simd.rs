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

    /// SIMD-parallel state for computing the Average Directional Movement Rating (ADXR) across
    /// `N` assets simultaneously. Each field is a SIMD vector where lane `i` corresponds to
    /// asset `i`.
    pub struct SimdState<const N: usize> {
        /// Embedded ADX SIMD state for all `N` asset lanes.
        pub adx_state: AdxSimdState<N>,
        /// Ring buffer that retains past ADX values used to compute the ADXR average.
        pub buffer: SimdBuffer<N>,
    }
    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each field into a SIMD lane.
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

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in
        /// place, avoiding allocation compared to a `to_states` conversion.
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

    /// Advances the ADXR by one bar for `N` assets simultaneously (checked variant).
    ///
    /// ADXR = `0.5 * (current_ADX + ADX_period_bars_ago)`. Returns zero for all lanes until
    /// enough bars have been processed to fill the internal ring buffer.
    ///
    /// # Returns
    ///
    /// A tuple `(adxr, adx, dx, atr, tr)` of SIMD vectors for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multipliers);

        let prev_adx = state.buffer.push_with_info(adx);
        let mut adxr = F64Constants::ZERO;
        if let Some(pa) = prev_adx {
            adxr = F64Constants::HALF * (adx + pa);
        }

        (adxr, adx, dx, atr, tr)
    }

    /// Advances the ADXR by one bar for `N` assets simultaneously (unchecked variant).
    ///
    /// # Safety
    ///
    /// The caller must guarantee the ring buffer already contains enough historical ADX values
    /// (i.e. the warm-up period has fully elapsed) before calling this function.
    #[inline(always)]
    pub unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multipliers);
        let adxr = F64Constants::HALF * (adx + state.buffer.push_with_info_unchecked(adx));

        (adxr, adx, dx, atr, tr)
    }
}

pub mod options {
    use super::imports::*;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{RingBuffer, UnsyncBuffer};

    /// SIMD-parallel state for computing the ADXR across `N` option lanes simultaneously.
    /// Uses per-lane ring buffers of potentially different periods stored in an `UnsyncBuffer`.
    pub struct SimdState<const N: usize> {
        /// Embedded ADX SIMD state for all `N` option lanes.
        pub adx_state: AdxSimdState<N>,
        /// Per-lane ring buffers with independent periods for each option set.
        pub buffer: UnsyncBuffer<N, f64>,
    }
    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each field into a SIMD lane.
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

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in
        /// place, avoiding allocation compared to a `to_states` conversion.
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

    /// Advances the ADXR by one bar for `N` option lanes simultaneously (checked variant).
    ///
    /// Each lane may have a different period; a SIMD mask gates lanes that are not yet warm.
    /// ADXR = `0.5 * (current_ADX + ADX_period_bars_ago)`.
    ///
    /// # Returns
    ///
    /// A tuple `(adxr, adx, dx, atr, tr)` of SIMD vectors for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multipliers);

        let (prev_adx, mask) = state.buffer.push_with_info(adx);

        let adxr = mask.select(F64Constants::HALF * (adx + prev_adx), F64Constants::ZERO);

        (adxr, adx, dx, atr, tr)
    }

    /// Advances the ADXR by one bar for `N` option lanes simultaneously (unchecked variant).
    ///
    /// # Safety
    ///
    /// The caller must guarantee all per-lane ring buffers are fully warmed up before calling.
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (adx, dx, atr, tr) = adx_calc_simd(&mut state.adx_state, high, low, close, multipliers);
        let adxr = F64Constants::HALF * (adx + state.buffer.push_with_info_unchecked(adx));

        (adxr, adx, dx, atr, tr)
    }
}
