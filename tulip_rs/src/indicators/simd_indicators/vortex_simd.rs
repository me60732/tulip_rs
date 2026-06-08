#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vortex::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vortex::indicator_by_options;

pub mod import {
    //! Internal imports shared by the [`assets`] and [`options`] SIMD sub-modules for the
    //! Vortex indicator.
    pub(crate) use crate::indicators::vortex::IndicatorState as State;
    pub(crate) use crate::ring_buffer::multi_buffer::multi_buffer::{MultiBuffer, RingBuffer};
    pub(crate) use crate::ring_buffer::multi_type_buffer::MultiTypeBuffer;
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}

pub mod assets {
    //! Per-asset SIMD state and compute for the Vortex indicator.
    use super::import::*;
    use crate::indicators::simd_indicators::tr_simd::calc_simd as tr_calc_simd;
    /// SIMD-parallel state for the Vortex indicator, holding `N` lanes of per-asset state.
    ///
    /// The internal buffer uses 3 channels — `[tr, vm_up, vm_down]` — stored as `Simd<f64, N>`
    /// per slot, so every ring-buffer operation advances all `N` assets simultaneously.
    ///
    /// ## Buffer bridge
    /// The single-asset [`State`] uses `MultiTypeBuffer<(f64, Simd<f64, 2>)>` (heterogeneous
    /// element types), which is incompatible with `SimdRingBuffer::from_f64_buffers`. The
    /// [`new`](SimdState::new) / [`write_states`](SimdState::write_states) pair therefore bridges
    /// via `to_ordered_vecs` + `RingBuffer::from_slice` — a one-time cost per epoch that is
    /// completely amortised over the hot loop.
    pub struct SimdState<const N: usize> {
        /// Ring buffer: 3 channels `[tr, vm_up, vm_down]`, each a `Simd<f64, N>` per slot.
        buffer: MultiBuffer<3, Simd<f64, N>>,
        vm_up_sums: Simd<f64, N>,
        vm_down_sums: Simd<f64, N>,
        tr_sums: Simd<f64, N>,
        /// Previous bar's low for each asset — used by `vm_up = |high − prev_low|`.
        prev_lows: Simd<f64, N>,
        /// Previous bar's high for each asset — used by `vm_down = |low − prev_high|`.
        prev_highs: Simd<f64, N>,
        prev_closes: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        /// Constructs a [`SimdState`] by merging `N` scalar [`State`] instances into SIMD lanes.
        ///
        /// Each single-asset buffer is drained in chronological order via `to_ordered_vecs`, then
        /// transposed from `N × period` into `3 × period` (channels × slots) and loaded into a
        /// `MultiBuffer<3, Simd<f64, N>>` via `RingBuffer::from_slice`.
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let period = states[0].buffer.get_capacity();
            let count = states[0].buffer.get_count();

            // to_ordered_vecs() → (Vec<f64>, Vec<Simd<f64,2>>) in chronological order,
            // where .0 = tr values, .1 = [vm_up, vm_down] values.
            let ordered: Vec<_> = states.iter().map(|s| s.buffer.to_ordered_vecs()).collect();

            // Transpose: N assets × count slots → 3 channel Vecs of Simd<f64, N>.
            let mut tr_ch: Vec<Simd<f64, N>> = Vec::with_capacity(count);
            let mut vm_up_ch: Vec<Simd<f64, N>> = Vec::with_capacity(count);
            let mut vm_dn_ch: Vec<Simd<f64, N>> = Vec::with_capacity(count);

            for slot in 0..count {
                tr_ch.push(Simd::from_array(core::array::from_fn(|a| {
                    ordered[a].0[slot]
                })));
                vm_up_ch.push(Simd::from_array(core::array::from_fn(|a| {
                    ordered[a].1[slot][0]
                })));
                vm_dn_ch.push(Simd::from_array(core::array::from_fn(|a| {
                    ordered[a].1[slot][1]
                })));
            }

            let buffer = <MultiBuffer<3, Simd<f64, N>> as RingBuffer<3, Simd<f64, N>>>::from_slice(
                [&tr_ch, &vm_up_ch, &vm_dn_ch],
                period,
            );

            let mut prev_closes = [0.0f64; N];
            let mut tr_sums = [0.0f64; N];
            let mut vm_up_sums = [0.0f64; N];
            let mut vm_down_sums = [0.0f64; N];
            let mut prev_lows = [0.0f64; N];
            let mut prev_highs = [0.0f64; N];

            for (i, s) in states.iter().enumerate() {
                prev_closes[i] = s.prev_close;
                tr_sums[i] = s.tr_sum;
                vm_up_sums[i] = s.vm_sums[0];
                vm_down_sums[i] = s.vm_sums[1];
                prev_lows[i] = s.prev_low_high[0];
                prev_highs[i] = s.prev_low_high[1];
            }

            Self {
                buffer,
                prev_closes: Simd::from_array(prev_closes),
                tr_sums: Simd::from_array(tr_sums),
                vm_up_sums: Simd::from_array(vm_up_sums),
                vm_down_sums: Simd::from_array(vm_down_sums),
                prev_lows: Simd::from_array(prev_lows),
                prev_highs: Simd::from_array(prev_highs),
            }
        }

        /// Writes SIMD state back into `N` scalar [`State`] references.
        ///
        /// Splits the `MultiBuffer<3, Simd<f64, N>>` lanes back into individual
        /// `MultiTypeBuffer<(f64, Simd<f64, 2>)>` buffers by extracting each asset's lane from
        /// the ordered SIMD slots.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // to_ordered_vec() → [Vec<Simd<f64,N>>; 3]: channels [tr, vm_up, vm_down],
            // each vec is in chronological order (oldest → newest).
            let ordered = self.buffer.to_ordered_vec();
            let count = self.buffer.get_count();
            let period = self.buffer.get_capacity();

            let prev_closes = self.prev_closes.to_array();
            let tr_sums = self.tr_sums.to_array();
            let vm_up_sums = self.vm_up_sums.to_array();
            let vm_down_sums = self.vm_down_sums.to_array();
            let prev_lows = self.prev_lows.to_array();
            let prev_highs = self.prev_highs.to_array();

            for asset in 0..N {
                let mut buf = MultiTypeBuffer::<(f64, Simd<f64, 2>)>::new(period);
                for slot in 0..count {
                    buf.push((
                        ordered[0][slot][asset],
                        Simd::from_array([ordered[1][slot][asset], ordered[2][slot][asset]]),
                    ));
                }

                states[asset].buffer = buf;
                states[asset].prev_close = prev_closes[asset];
                states[asset].tr_sum = tr_sums[asset];
                states[asset].vm_sums = Simd::from_array([vm_up_sums[asset], vm_down_sums[asset]]);
                states[asset].prev_low_high =
                    Simd::from_array([prev_lows[asset], prev_highs[asset]]);
            }
        }

        /// Computes one Vortex bar for `N` assets simultaneously.
        ///
        /// Returns `(vi_up, vi_down)` for all `N` asset lanes.
        ///
        /// # Safety
        ///
        /// The internal ring buffer must be fully initialised (at least `period` bars processed)
        /// before calling this function. Calling it on a partial buffer produces incorrect results.
        #[inline(always)]
        pub unsafe fn calc_unchecked(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            close: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let tr = tr_calc_simd(high, low, self.prev_closes);

            // Vortex movements: vm_up = |high − prev_low|, vm_down = |low − prev_high|
            let vm_up = (high - self.prev_lows).abs();
            let vm_dn = (low - self.prev_highs).abs();

            let [old_tr, old_vm_up, old_vm_dn] =
                self.buffer.push_with_info_unchecked([tr, vm_up, vm_dn]);

            self.tr_sums += tr - old_tr;
            self.vm_up_sums += vm_up - old_vm_up;
            self.vm_down_sums += vm_dn - old_vm_dn;

            // Update prev values after computing vm (order matters).
            self.prev_closes = close;
            self.prev_lows = low;
            self.prev_highs = high;

            (
                self.vm_up_sums / self.tr_sums,
                self.vm_down_sums / self.tr_sums,
                tr,
            )
        }
    }
}

pub mod options {
    //! Per-option SIMD state and compute for the Vortex indicator.
    //!
    //! All `N` option lanes share a single asset's price stream; each lane uses a different
    //! `period`. A single `MultiBuffer<3, f64>` sized to the longest period holds the full
    //! history; `push_with_info_periods_unchecked` retrieves each lane's rolling window boundary
    //! in one pass — the same shared-buffer pattern used by multi-period indicators such as ULTOSC.
    use super::import::*;
    use crate::indicators::tr::calc as calc_tr;
    /// SIMD-parallel state for the Vortex indicator, holding `N` lanes of per-option state.
    ///
    /// Because all lanes process the same asset, `prev_low`, `prev_high`, and `prev_close`
    /// are shared scalars. The per-lane sums (`tr_sums`, `vm_up_sums`, `vm_down_sums`) are
    /// `Simd<f64, N>`.
    pub struct SimdState<const N: usize> {
        /// Shared ring buffer sized to `max(periods)`, channels `[tr, vm_up, vm_down]`.
        buffer: MultiBuffer<3>,
        periods: [usize; N],
        vm_up_sums: Simd<f64, N>,
        vm_down_sums: Simd<f64, N>,
        tr_sums: Simd<f64, N>,
        /// Scalar — shared across lanes because all lanes process the same asset.
        pub prev_low_high: Simd<f64, 2>,
        prev_close: f64,
    }

    impl<const N: usize> SimdState<N> {
        /// Constructs a [`SimdState`] from `N` scalar [`State`] references (one per period lane).
        ///
        /// Selects the state with the longest period as the shared buffer base, converts its
        /// `MultiTypeBuffer<(f64, Simd<f64, 2>)>` to a `MultiBuffer<3, f64>` via
        /// `to_ordered_vecs` + `RingBuffer::from_slice`, then gathers each lane's rolling sums
        /// into SIMD vectors.
        pub fn new(states: &mut [&mut State], periods: [usize; N]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Find the state with the largest period (longest buffer).
            let mut main = 0;
            for i in 1..N {
                if states[main].buffer.get_capacity() < states[i].buffer.get_capacity() {
                    main = i;
                }
            }

            // Convert MultiTypeBuffer<(f64, Simd<f64,2>)> → MultiBuffer<3, f64>.
            // to_ordered_vecs() → (Vec<f64>, Vec<Simd<f64,2>>) in chronological order.
            let ordered = states[main].buffer.to_ordered_vecs();
            let tr_vec: Vec<f64> = ordered.0;
            let vm_up_vec: Vec<f64> = ordered.1.iter().map(|v| v[0]).collect();
            let vm_dn_vec: Vec<f64> = ordered.1.iter().map(|v| v[1]).collect();
            let period = states[main].buffer.get_capacity();

            let buffer = <MultiBuffer<3, f64> as RingBuffer<3, f64>>::from_slice(
                [&tr_vec, &vm_up_vec, &vm_dn_vec],
                period,
            );

            let prev_close = states[main].prev_close;
            let prev_low_high = states[main].prev_low_high;

            let mut tr_sums = [0.0f64; N];
            let mut vm_up_sums = [0.0f64; N];
            let mut vm_down_sums = [0.0f64; N];

            for (i, s) in states.iter().enumerate() {
                tr_sums[i] = s.tr_sum;
                vm_up_sums[i] = s.vm_sums[0];
                vm_down_sums[i] = s.vm_sums[1];
            }

            Self {
                buffer,
                periods,
                prev_close,
                prev_low_high,
                tr_sums: Simd::from_array(tr_sums),
                vm_up_sums: Simd::from_array(vm_up_sums),
                vm_down_sums: Simd::from_array(vm_down_sums),
            }
        }

        /// Writes SIMD state back into `N` scalar [`State`] references.
        ///
        /// Slices the shared buffer to each lane's own period via `to_ordered_by_period`, then
        /// repacks each slice into a `MultiTypeBuffer<(f64, Simd<f64, 2>)>`.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // Slice the shared buffer down to each lane's period in chronological order.
            // to_ordered_by_period(p) → [Vec<f64>; 3] of the `p` most recent entries.
            let ordered: [[Vec<f64>; 3]; N] =
                core::array::from_fn(|i| self.buffer.to_ordered_by_period(self.periods[i]));

            let tr_sums = self.tr_sums.to_array();
            let vm_up_sums = self.vm_up_sums.to_array();
            let vm_down_sums = self.vm_down_sums.to_array();

            for (i, ord) in ordered.into_iter().enumerate() {
                let count = ord[0].len(); // = periods[i] (or less if buffer was partially filled)
                let mut buf = MultiTypeBuffer::<(f64, Simd<f64, 2>)>::new(self.periods[i]);
                for slot in 0..count {
                    buf.push((ord[0][slot], Simd::from_array([ord[1][slot], ord[2][slot]])));
                }

                states[i].buffer = buf;
                states[i].tr_sum = tr_sums[i];
                states[i].vm_sums = Simd::from_array([vm_up_sums[i], vm_down_sums[i]]);
                states[i].prev_close = self.prev_close;
                states[i].prev_low_high = self.prev_low_high;
            }
        }

        /// Computes one Vortex bar for `N` option-set lanes simultaneously.
        ///
        /// Accepts scalar `high`, `low`, `close` (shared across all lanes — same asset) and
        /// returns `(vi_up, vi_down)` as SIMD vectors, one value per period lane.
        ///
        /// `push_with_info_periods_unchecked` pushes the new bar and retrieves each lane's
        /// rolling-window boundary value in a single pass, so no secondary `get_by_periods`
        /// call is needed.
        ///
        /// # Safety
        ///
        /// The internal ring buffer must be fully initialised (at least `max(periods)` bars
        /// processed) before calling this function.
        #[inline(always)]
        pub unsafe fn calc_unchecked(
            &mut self,
            high: f64,
            low: f64,
            close: f64,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let tr = calc_tr(high, low, self.prev_close);

            let high_low_simd = Simd::from_array([high, low]);
            let [vm_up, vm_dn] = ((high_low_simd - self.prev_low_high).abs()).to_array();
    
            self.prev_close = close;
            self.prev_low_high = high_low_simd.reverse();

            // Push new bar and pop each lane's oldest value (at that lane's period) in one pass.
            let [tr_old, vm_up_old, vm_dn_old] = self
                .buffer
                .push_with_info_periods_unchecked([tr, vm_up, vm_dn], self.periods);

            let tr_simd = Simd::splat(tr);
            self.tr_sums += tr_simd - Simd::from_array(tr_old);
            self.vm_up_sums += Simd::splat(vm_up) - Simd::from_array(vm_up_old);
            self.vm_down_sums += Simd::splat(vm_dn) - Simd::from_array(vm_dn_old);

            self.prev_close = close;

            (
                self.vm_up_sums / self.tr_sums,
                self.vm_down_sums / self.tr_sums,
                tr_simd,
            )
        }
    }
}
