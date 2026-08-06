#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::chaikinmf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::chaikinmf::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
pub(crate) mod imports {
    pub(crate) use crate::indicators::chaikinmf::IndicatorState as State;
    pub(crate) use crate::indicators::simd_indicators::simd_types::F64Constants;
    pub(crate) use crate::ring_buffer::multi_buffer::multi_buffer::MultiBuffer;
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}
use crate::types::Warm;
pub mod assets {
    use super::imports::*;
    use super::*;
    use crate::{indicator_types::TSimdState, ring_buffer::single_buffer::generic_buffer::Buffer};

    /// SIMD-parallel state for computing Chaikin Money Flow (CMF) across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2, Simd<f64, N>, Warm>,
        vol_sum: Simd<f64, N>,
        mfv_sum: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;

        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each asset's ring-buffer history and running sums into SIMD lanes.
        fn from_states(states: &mut [&mut State]) -> Self {
            let buffer_refs: [Vec<Simd<f64, 2>>; N] =
                core::array::from_fn(|i| states[i].buffer.to_ordered_vec());
            let mfv_sum: [f64; N] = core::array::from_fn(|i| states[i].sums[0]);
            let vol_sum: [f64; N] = core::array::from_fn(|i| states[i].sums[1]);

            let len = buffer_refs[0].len();
            let mut mfv_vals = Vec::<Simd<f64, N>>::with_capacity(len);
            let mut vol_vals = Vec::<Simd<f64, N>>::with_capacity(len);
            for i in 0..len {
                let mut mfv = [0.0; N];
                let mut volume = [0.0; N];
                for j in 0..N {
                    let [mfv_val, vol_val] = buffer_refs[j][i].to_array();
                    mfv[j] = mfv_val;
                    volume[j] = vol_val;
                }
                mfv_vals.push(Simd::from_array(mfv));
                vol_vals.push(Simd::from_array(volume));
            }

            let buffer = MultiBuffer {
                vals: [mfv_vals, vol_vals],
                index: 0,
                capacity: len,
                count: len,
                prev_idx: len - 1,
                state: std::marker::PhantomData,
            };

            Self {
                buffer,
                mfv_sum: Simd::from_array(mfv_sum),
                vol_sum: Simd::from_array(vol_sum),
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
        /// unpacking each lane's ring-buffer history and running sums.
        fn write_states(&self, states: &mut [&mut State]) {
            let mfv_sum = self.mfv_sum.to_array();
            let vol_sum = self.vol_sum.to_array();
            let capacity = self.buffer.capacity;

            for n in 0..N {
                let mut packed = Vec::with_capacity(capacity);
                for t in 0..capacity {
                    let mfv = self.buffer.vals[0][t].to_array()[n];
                    let vol = self.buffer.vals[1][t].to_array()[n];
                    packed.push(Simd::from_array([mfv, vol]));
                }
                states[n].buffer = Buffer {
                    vals: packed,
                    index: self.buffer.index,
                    capacity,
                    count: self.buffer.count,
                    prev_idx: self.buffer.prev_idx,
                    state: std::marker::PhantomData::<Warm>,
                };
                states[n].sums = Simd::from_array([mfv_sum[n], vol_sum[n]]);
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;
        /// Computes one CMF step across `N` asset lanes using SIMD parallelism.
        ///
        /// Calculates `mfv = ((close - low) - (high - close)) / (high - low) * volume`
        /// for each lane, pushes `[mfv, volume]` into the shared ring buffer, updates
        /// the rolling `mfv_sum` and `vol_sum`, and returns `mfv_sum / vol_sum`.
        ///
        /// # Safety
        /// Caller must ensure all `N` ring buffers have been fully seeded.
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
            let mfv = ((close - low) - (high - close))
                / (high - low).simd_max(F64Constants::EPSILON)
                * volume;

            let [old_mfv, old_vol] = self.buffer.push_with_info([mfv, volume]);
            self.vol_sum += volume - old_vol;
            self.mfv_sum += mfv - old_mfv;

            self.mfv_sum / self.vol_sum
        }
    }
}

pub mod options {
    use super::imports::*;
    use super::*;
    use crate::ring_buffer::single_buffer::generic_buffer::Buffer;

    /// SIMD-parallel state for computing Chaikin Money Flow across `N` option lanes
    /// (different periods) on a single asset simultaneously.
    ///
    /// The buffer is sized to the widest period and shared across all lanes;
    /// each lane reads back its own period via `push_with_info_periods_unchecked`.
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2, f64, Warm>,
        mfv_sum: Simd<f64, N>,
        vol_sum: Simd<f64, N>,
        periods: [usize; N],
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`State`] references (each with a potentially different period)
        /// into a single `SimdState`, using the widest period's buffer as the shared ring buffer.
        ///
        /// Running sums for each lane are copied from the corresponding scalar state.
        pub fn from_states(states: &mut [&mut State], periods: [usize; N]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            // Use the widest-period buffer as the shared multi-buffer
            let mut main = 0;
            for i in 1..N {
                if states[i].buffer.capacity > states[main].buffer.capacity {
                    main = i;
                }
            }

            // Convert Buffer<Simd<f64, 2>> → MultiBuffer<2, f64> by splitting lanes
            let ordered: Vec<Simd<f64, 2>> = states[main].buffer.to_ordered_vec();
            let capacity = states[main].buffer.capacity;
            let mut mfv_band = Vec::with_capacity(capacity);
            let mut vol_band = Vec::with_capacity(capacity);
            for v in &ordered {
                let [mfv, vol] = v.to_array();
                mfv_band.push(mfv);
                vol_band.push(vol);
            }
            let buffer = MultiBuffer {
                vals: [mfv_band, vol_band],
                index: 0,
                prev_idx: capacity - 1,
                capacity,
                count: capacity,
                state: std::marker::PhantomData::<Warm>,
            };

            let mfv_sum: [f64; N] = core::array::from_fn(|i| states[i].sums[0]);
            let vol_sum: [f64; N] = core::array::from_fn(|i| states[i].sums[1]);

            Self {
                buffer,
                mfv_sum: Simd::from_array(mfv_sum),
                vol_sum: Simd::from_array(vol_sum),
                periods,
            }
        }

        /// Writes the SIMD state back into `N` scalar [`State`] references in place.
        ///
        /// Each lane's period-specific slice is extracted from the shared ring buffer
        /// and packed back into a `Buffer<Simd<f64, 2>>` with the correct running sums.
        pub fn write_states(&self, states: &mut [&mut State]) {
            let mfv_sum = self.mfv_sum.to_array();
            let vol_sum = self.vol_sum.to_array();

            for i in 0..N {
                // Get the period-specific ordered slice for each band
                let [mfv_ordered, vol_ordered] = self.buffer.to_ordered_by_period(self.periods[i]);
                let capacity = mfv_ordered.len();

                // Zip bands back into packed Buffer<Simd<f64, 2>>
                let mut packed = Vec::with_capacity(capacity);
                for (&mfv, &vol) in mfv_ordered.iter().zip(vol_ordered.iter()) {
                    packed.push(Simd::from_array([mfv, vol]));
                }
                states[i].buffer = Buffer {
                    vals: packed,
                    index: 0,
                    prev_idx: capacity - 1,
                    capacity,
                    count: capacity,
                    state: std::marker::PhantomData::<Warm>,
                };
                states[i].sums = Simd::from_array([mfv_sum[i], vol_sum[i]]);
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64, f64, f64);
        type Outputs = Simd<f64, N>;

        /// Computes one Chaikin MF step for `N` period lanes on a single scalar bar.
        ///
        /// # Safety
        /// Caller must ensure the buffer has capacity for one more element.
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
            let mfv = ((close - low) - (high - close)) / (high - low).max(f64::EPSILON) * volume;

            let [mfv_old, vol_old] = self
                .buffer
                .push_with_info_periods([mfv, volume], self.periods);
            self.mfv_sum += Simd::splat(mfv) - Simd::from_array(mfv_old);
            self.vol_sum += Simd::splat(volume) - Simd::from_array(vol_old);

            self.mfv_sum / self.vol_sum.simd_max(Simd::splat(f64::EPSILON))
        }
    }
}
