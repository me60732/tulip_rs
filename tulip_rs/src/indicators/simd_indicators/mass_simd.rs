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

/// Asset-parallel SIMD computations for the Mass Index.
///
/// Provides [`SimdState`] for gathering `N` scalar states into SIMD lanes, advancing one
/// bar of Mass Index across all lanes simultaneously, and scattering results back to scalars.
pub mod asset {
    use super::imports::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };

    /// SIMD-parallel state for computing the Mass Index across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        /// Ring buffer holding the per-bar mass values (EMA/EMA_signal ratio) over the rolling sum window.
        pub buffer: SimdBuffer<N>,
        /// Running sum of mass values over the current period window per lane.
        pub sum: Simd<f64, N>,
        /// Current 9-period EMA of (High - Low) per asset lane.
        pub ema: Simd<f64, N>,
        /// Current 9-period EMA of the EMA (signal smoothing) per asset lane.
        pub ema_signal: Simd<f64, N>,
    }
    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
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
        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
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
        /// Computes one Mass Index step across `N` asset lanes using SIMD parallelism.
        ///
        /// Advances the EMA and signal-EMA of (High - Low), computes the EMA ratio (mass),
        /// then maintains a rolling sum over the period window via the ring buffer.
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
        /// Like [`calc_simd`](Self::calc_simd) but skips ring-buffer bounds checks.
        ///
        /// # Safety
        /// The caller must guarantee the buffer has sufficient capacity for one additional element.
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
    /// Advances the EMA and signal-EMA of `(high - low)` by one bar and returns `(mass, new_ema, new_ema_signal)`.
    ///
    /// `mass = (ema / ema_signal).max(0.0)`. The result is clamped to avoid negative values
    /// from floating-point noise. This pure helper is shared by both
    /// [`SimdState::calc_simd`] and [`SimdState::calc_unchecked_simd`].
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

/// Option-parallel SIMD computations for the Mass Index with `N` different period settings on a single asset.
///
/// Shares the EMA/signal-EMA price state across all `N` lanes while maintaining per-lane
/// rolling sums over each lane's individual period.
pub mod option {
    use super::imports::*;
    use crate::indicators::ema::calc as ema_calc;
    use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};

    /// State for computing the Mass Index with `N` different period options on a single asset.
    ///
    /// Each lane `i` has its own period and running sum, but the EMA/signal-EMA scalars are shared
    /// (computed from the same price series) and the ring buffer is sized to the largest period.
    pub struct SimdState<const N: usize> {
        /// Shared ring buffer sized to the maximum period across all `N` option lanes.
        pub buffer: Buffer,
        /// Per-lane rolling sum of mass values over each lane's individual period.
        pub sum: Simd<f64, N>,
        /// Shared 9-period EMA of (High - Low) (scalar, same series for all lanes).
        pub ema: f64,
        /// Shared 9-period signal-EMA of the EMA (scalar, same series for all lanes).
        pub ema_signal: f64,
        periods: [usize; N],
    }
    impl<const N: usize> SimdState<N> {
        /// Initialises the option-mode state by borrowing `N` scalar [`State`] references.
        ///
        /// Picks the largest buffer (widest period) as the shared buffer and packs `sum` per lane.
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
        /// Writes the option-mode SIMD state back into `N` existing mutable scalar [`State`] references.
        ///
        /// Re-slices the shared buffer to each lane's period so each scalar state gets
        /// the correct ordered window.
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

        /// Computes one Mass Index step for `N` option lanes on a single scalar bar.
        ///
        /// Advances the shared EMA/signal pair, computes the mass, pushes it into the
        /// shared buffer, then deducts the evicted values per lane's individual period.
        ///
        /// # Safety
        /// Caller must ensure the buffer has capacity for one more element.
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
