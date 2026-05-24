#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::hma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::hma::indicator_by_options;

mod imports {
    pub(crate) use crate::indicators::hma::State;
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants,
        wma_simd::{calc_simd as wma_calc_simd, SimdState as WmaSimdState},
    };
    pub(crate) use std::simd::{Select, Simd, StdFloat};
}

pub mod assets {
    //! Per-asset road SIMD state and compute functions for the Hull Moving Average (HMA) indicator.
    use super::imports::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };
    /// SIMD-parallel state for the Hull Moving Average (HMA) indicator (per-asset variant),
    /// holding `N` lanes of per-asset state.
    pub struct SimdState<const N: usize> {
        pub prev_diff: SimdBuffer<N>,
        pub state1: WmaSimdState<N>,
        pub state2: WmaSimdState<N>,
        pub weighted_sumsqrt: Simd<f64, N>,
        pub sumsqrt: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Build buffer array directly (immutable references are fine)

            // Build ADX refs using indexing instead of iterator
            let mut state1_refs = Vec::with_capacity(N);
            let mut state2_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            let mut weighted_sumsqrt = [0.0; N];
            let mut sumsqrt = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                state1_refs.push(&mut state.state1);
                state2_refs.push(&mut state.state2);
                buffer_refs.push(&state.prev_diff); //.to_ordered_vec());
                weighted_sumsqrt[i] = state.weighted_sumsqrt;
                sumsqrt[i] = state.sumsqrt;
            }

            let state1 = WmaSimdState::new(&mut state1_refs);
            let state2 = WmaSimdState::new(&mut state2_refs);
            let prev_diff = SimdBuffer::from_f64_buffers(buffer_refs);

            Self {
                state1,
                state2,
                prev_diff,
                weighted_sumsqrt: Simd::from_array(weighted_sumsqrt),
                sumsqrt: Simd::from_array(sumsqrt),
            }
        }

        /// Writes the current SIMD lane values back into the provided scalar per-asset states.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.prev_diff.to_f64_buffers();
            let weighted_sumsqrt = self.weighted_sumsqrt.to_array();
            let sumsqrt = self.sumsqrt.to_array();

            // Now collect WMA references using iter_mut() instead of indexing
            let mut state1_refs = Vec::with_capacity(N);
            let mut state2_refs = Vec::with_capacity(N);

            for (i, (state, buffer)) in states.iter_mut().zip(buffers.into_iter()).enumerate() {
                state1_refs.push(&mut state.state1);
                state2_refs.push(&mut state.state2);
                state.weighted_sumsqrt = weighted_sumsqrt[i];
                state.sumsqrt = sumsqrt[i];
                state.prev_diff = buffer;
            }

            // Finally, update the WMA states
            self.state1.write_states(&mut state1_refs);
            self.state2.write_states(&mut state2_refs);
        }
    }

    /// Computes one bar of the Hull Moving Average (HMA) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Combines two WMAs of differing period lengths, computes their weighted difference,
    /// and then applies a final WMA over a sqrt-period window.
    /// Returns zero until the sqrt-period ring buffer is full.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable SIMD state.
    /// * `value` - Current prices for this bar.
    /// * `prev_value` - Oldest value dropped from the half-period WMA window.
    /// * `prev_value2` - Oldest value dropped from the full-period WMA window.
    /// * `multipliers` - Tuple `(sqrt_period, weights_sqrt, half_period_multipliers, full_period_multipliers)`.
    ///
    /// # Returns
    ///
    /// HMA values for all `N` lanes (zero until warm-up is complete).
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        prev_value2: Simd<f64, N>,
        multipliers: (
            Simd<f64, N>,
            Simd<f64, N>,
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> Simd<f64, N> {
        let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
        let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);

        let (wma, _) = wma_calc_simd(&mut state.state1, prev_value, value, multiplier);

        let (wma2, _) = wma_calc_simd(&mut state.state2, prev_value2, value, multiplier2);

        //let diff = F64Constants::TWO * wma2 - wma;
        let diff = wma2.mul_add(F64Constants::TWO, -wma);
        weighted_sumsqrt = diff.mul_add(periodsqrt, weighted_sumsqrt);
        sumsqrt += diff;

        let prev_diff = &mut state.prev_diff;
        prev_diff.push(diff);

        let mut hma = F64Constants::ZERO;

        if prev_diff.is_full() {
            hma = weighted_sumsqrt / weightssqrt;
            weighted_sumsqrt -= sumsqrt;
            sumsqrt -= unsafe { prev_diff.front_unchecked() };
        } else {
            weighted_sumsqrt -= sumsqrt;
        }
        (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);
        hma
    }
    /// Computes one bar of the HMA for `N` assets simultaneously without buffer-fullness checks
    /// (unsafe, unchecked variant — the ring buffer must be full before calling).
    ///
    /// # Safety
    /// The internal `prev_diff` ring buffer must already be full; calling before warm-up
    /// is complete will produce incorrect results or undefined behaviour.
    #[inline(always)]
    pub unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        prev_value2: Simd<f64, N>,
        multipliers: (
            Simd<f64, N>,
            Simd<f64, N>,
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> Simd<f64, N> {
        let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
        let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);

        let (wma, _) = wma_calc_simd(&mut state.state1, prev_value, value, multiplier);

        let (wma2, _) = wma_calc_simd(&mut state.state2, prev_value2, value, multiplier2);

        //let diff = F64Constants::TWO * wma2 - wma;
        let diff = wma2.mul_add(F64Constants::TWO, -wma);
        //weighted_sumsqrt += diff * periodsqrt;
        weighted_sumsqrt = diff.mul_add(periodsqrt, weighted_sumsqrt);
        sumsqrt += diff;

        let prev_diff = &mut state.prev_diff;
        prev_diff.push_unchecked(diff);

        let hma = weighted_sumsqrt / weightssqrt;
        weighted_sumsqrt -= sumsqrt;
        sumsqrt -= prev_diff.front_unchecked();
        (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);

        hma
    }
}

pub mod options {
    //! Per-option road SIMD state and compute functions for the Hull Moving Average (HMA) indicator.
    use super::imports::*;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{RingBuffer, UnsyncBuffer};

    /// SIMD-parallel state for the Hull Moving Average (HMA) indicator (per-option variant),
    /// holding `N` lanes of per-option state.
    pub struct SimdState<const N: usize> {
        pub prev_diff: UnsyncBuffer<N, f64>,
        pub state1: WmaSimdState<N>,
        pub state2: WmaSimdState<N>,
        pub weighted_sumsqrt: Simd<f64, N>,
        pub sumsqrt: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Constructs a `SimdState` by gathering scalar per-option states into SIMD vectors.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut state1_refs = Vec::with_capacity(N);
            let mut state2_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            let mut weighted_sumsqrt = [0.0; N];
            let mut sumsqrt = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                state1_refs.push(&mut state.state1);
                state2_refs.push(&mut state.state2);
                buffer_refs.push(&state.prev_diff);
                weighted_sumsqrt[i] = state.weighted_sumsqrt;
                sumsqrt[i] = state.sumsqrt;
            }

            let state1 = WmaSimdState::new(&mut state1_refs);
            let state2 = WmaSimdState::new(&mut state2_refs);
            let prev_diff = UnsyncBuffer::from_buffers(buffer_refs);

            Self {
                state1,
                state2,
                prev_diff,
                weighted_sumsqrt: Simd::from_array(weighted_sumsqrt),
                sumsqrt: Simd::from_array(sumsqrt),
            }
        }

        /// Writes the current SIMD lane values back into the provided scalar per-option states.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.prev_diff.to_f64_buffers();
            let weighted_sumsqrt = self.weighted_sumsqrt.to_array();
            let sumsqrt = self.sumsqrt.to_array();

            // Now collect WMA references using iter_mut() instead of indexing
            let mut state1_refs = Vec::with_capacity(N);
            let mut state2_refs = Vec::with_capacity(N);

            for (i, (state, buffer)) in states.iter_mut().zip(buffers.into_iter()).enumerate() {
                state1_refs.push(&mut state.state1);
                state2_refs.push(&mut state.state2);
                state.weighted_sumsqrt = weighted_sumsqrt[i];
                state.sumsqrt = sumsqrt[i];
                state.prev_diff = buffer;
            }

            // Finally, update the WMA states
            self.state1.write_states(&mut state1_refs);
            self.state2.write_states(&mut state2_refs);
        }
    }

    /// Computes one bar of the Hull Moving Average (HMA) for `N` option lanes simultaneously
    /// using SIMD parallelism.
    ///
    /// Identical logic to the assets variant but uses a lock-free `UnsyncBuffer`.
    /// Returns zero until the sqrt-period ring buffer is full.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable SIMD state.
    /// * `value` - Current prices for this bar.
    /// * `prev_value` - Oldest value dropped from the half-period WMA window.
    /// * `prev_value2` - Oldest value dropped from the full-period WMA window.
    /// * `multipliers` - Tuple `(sqrt_period, weights_sqrt, half_period_multipliers, full_period_multipliers)`.
    ///
    /// # Returns
    ///
    /// HMA values for all `N` lanes (zero until warm-up is complete).
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        state: &mut SimdState<N>,
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        prev_value2: Simd<f64, N>,
        multipliers: (
            Simd<f64, N>,
            Simd<f64, N>,
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> Simd<f64, N> {
        let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
        let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);

        let (wma, _) = wma_calc_simd(&mut state.state1, prev_value, value, multiplier);

        let (wma2, _) = wma_calc_simd(&mut state.state2, prev_value2, value, multiplier2);

        let diff = wma2.mul_add(F64Constants::TWO, -wma);
        //weighted_sumsqrt += diff * periodsqrt;
        weighted_sumsqrt = diff.mul_add(periodsqrt, weighted_sumsqrt);
        sumsqrt += diff;

        let prev_diff = &mut state.prev_diff;
        prev_diff.push(diff);

        let mask = prev_diff.is_full();
        let hma = mask.select(weighted_sumsqrt / weightssqrt, F64Constants::ZERO);
        sumsqrt = mask.select(sumsqrt - prev_diff.front_unchecked(), sumsqrt);
        weighted_sumsqrt -= sumsqrt;

        (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);
        hma
    }
    /// Computes one bar of the HMA for `N` option lanes without buffer-fullness checks
    /// (unsafe, unchecked variant — the ring buffer must be full before calling).
    ///
    /// # Safety
    /// The internal `prev_diff` ring buffer must already be full; calling before warm-up
    /// is complete will produce incorrect results or undefined behaviour.
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked_simd<const N: usize>(
        state: &mut SimdState<N>,
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        prev_value2: Simd<f64, N>,
        multipliers: (
            Simd<f64, N>,
            Simd<f64, N>,
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> Simd<f64, N> {
        let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
        let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);

        let (wma, _) = wma_calc_simd(&mut state.state1, prev_value, value, multiplier);

        let (wma2, _) = wma_calc_simd(&mut state.state2, prev_value2, value, multiplier2);

        let diff = wma2.mul_add(F64Constants::TWO, -wma);
        //weighted_sumsqrt += diff * periodsqrt;
        weighted_sumsqrt = diff.mul_add(periodsqrt, weighted_sumsqrt);
        sumsqrt += diff;

        let prev_diff = &mut state.prev_diff;
        prev_diff.push_unchecked(diff);

        let hma = weighted_sumsqrt / weightssqrt;
        weighted_sumsqrt -= sumsqrt;
        sumsqrt -= prev_diff.front_unchecked();
        (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);

        hma
    }
}
