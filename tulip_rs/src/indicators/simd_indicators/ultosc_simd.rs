#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ultosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ultosc::indicator_by_options;

pub mod import {
    pub(crate) use crate::indicators::simd_indicators::simd_types::F64Constants;
    pub(crate) use crate::indicators::ultosc::State;
    pub(crate) use crate::ring_buffer::multi_buffer::multi_buffer::{MultiBuffer, RingBuffer};
    pub(crate) use std::simd::{num::SimdFloat, Simd};
    pub(crate) struct UltoscF64Constants<const N: usize>;

    impl<const N: usize> UltoscF64Constants<N> {
        pub const DIV: Simd<f64, N> = Simd::splat(100.0 / 7.0);
    }
}

pub mod assets {
    use super::import::*;
    pub(crate) use crate::ring_buffer::multi_buffer::multi_buffer::SimdRingBuffer;
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2, Simd<f64, N>>,

        bp_short_sum: Simd<f64, N>,
        bp_medium_sum: Simd<f64, N>,
        bp_long_sum: Simd<f64, N>,

        tr_short_sum: Simd<f64, N>,
        tr_medium_sum: Simd<f64, N>,
        tr_long_sum: Simd<f64, N>,

        prev_close: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let buffer_refs: [&MultiBuffer<2, f64>; N] =
                core::array::from_fn(|i| &states[i].buffer);
            let buffer = <MultiBuffer<2, Simd<f64, N>> as SimdRingBuffer<2, N>>::from_f64_buffers(
                buffer_refs,
            );

            let mut bp_short_sum = [0.0; N];
            let mut bp_medium_sum = [0.0; N];
            let mut bp_long_sum = [0.0; N];

            let mut tr_short_sum = [0.0; N];
            let mut tr_medium_sum = [0.0; N];
            let mut tr_long_sum = [0.0; N];

            let mut prev_close = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                (bp_short_sum[i], bp_medium_sum[i], bp_long_sum[i]) =
                    (state.bp_sums_2x[0], state.bp_sums_2x[1], state.bp_long_sum);
                (tr_short_sum[i], tr_medium_sum[i], tr_long_sum[i]) =
                    (state.tr_sums_2x[0], state.tr_sums_2x[1], state.tr_long_sum);
                prev_close[i] = state.prev_close;
            }

            Self {
                buffer,
                bp_short_sum: Simd::from_array(bp_short_sum),
                bp_medium_sum: Simd::from_array(bp_medium_sum),
                bp_long_sum: Simd::from_array(bp_long_sum),
                tr_short_sum: Simd::from_array(tr_short_sum),
                tr_medium_sum: Simd::from_array(tr_medium_sum),
                tr_long_sum: Simd::from_array(tr_long_sum),
                prev_close: Simd::from_array(prev_close),
            }
        }

        pub fn to_states(&self) -> [State; N] {
            let buffers = self.buffer.to_f64_buffers();
            let bp_short_sum = self.bp_short_sum.to_array();
            let bp_medium_sum = self.bp_medium_sum.to_array();
            let bp_long_sum = self.bp_long_sum.to_array();
            let tr_short_sum = self.tr_short_sum.to_array();
            let tr_medium_sum = self.tr_medium_sum.to_array();
            let tr_long_sum = self.tr_long_sum.to_array();
            let prev_close = self.prev_close.to_array();
            // Use into_iter() to consume the arrays and avoid move issues
            let mut states_vec = Vec::<State>::with_capacity(N);
            for (i, buffer) in buffers.into_iter().enumerate() {
                states_vec.push(State {
                    buffer,
                    bp_long_sum: bp_long_sum[i],
                    bp_sums_2x: Simd::<f64, 2>::from_array([bp_short_sum[i], bp_medium_sum[i]]),
                    tr_long_sum: tr_long_sum[i],
                    tr_sums_2x: Simd::<f64, 2>::from_array([tr_short_sum[i], tr_medium_sum[i]]),
                    prev_close: prev_close[i],
                });
            }

            // Convert Vec to array
            states_vec
                .try_into()
                .unwrap_or_else(|_| panic!("Failed to convert states_vec to array"))
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let bp_short_sum = self.bp_short_sum.to_array();
            let bp_medium_sum = self.bp_medium_sum.to_array();
            let bp_long_sum = self.bp_long_sum.to_array();
            let tr_short_sum = self.tr_short_sum.to_array();
            let tr_medium_sum = self.tr_medium_sum.to_array();
            let tr_long_sum = self.tr_long_sum.to_array();
            let prev_close = self.prev_close.to_array();

            for (i, buffer) in buffers.into_iter().enumerate() {
                states[i].buffer = buffer;
                (
                    states[i].bp_sums_2x[0],
                    states[i].bp_sums_2x[1],
                    states[i].bp_long_sum,
                ) = (bp_short_sum[i], bp_medium_sum[i], bp_long_sum[i]);
                (
                    states[i].tr_sums_2x[0],
                    states[i].tr_sums_2x[1],
                    states[i].tr_long_sum,
                ) = (tr_short_sum[i], tr_medium_sum[i], tr_long_sum[i]);
                states[i].prev_close = prev_close[i];
            }
        }

        #[inline(always)]
        pub fn calc(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            close: Simd<f64, N>,
            periods: (usize, usize),
        ) -> Simd<f64, N> {
            let (short_period, medium_period) = periods;

            let true_low = low.simd_min(self.prev_close);
            let true_high = high.simd_max(self.prev_close);
            let bp = close - true_low;
            let tr = true_high - true_low;

            if let Some(old) = self.buffer.push_with_info([bp, tr]) {
                self.bp_long_sum += bp - old[0];
                self.tr_long_sum += tr - old[1];
            } else {
                self.bp_long_sum += bp;
                self.tr_long_sum += tr;
            }
            let [[bp_short, bp_medium], [tr_short, tr_medium]] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            self.bp_short_sum += bp - bp_short;
            self.bp_medium_sum += bp - bp_medium;
            self.tr_short_sum += tr - tr_short;
            self.tr_medium_sum += tr - tr_medium;

            self.prev_close = close;

            if self.buffer.is_full() {
                let first = F64Constants::FOUR * (self.bp_short_sum / self.tr_short_sum);
                let second = F64Constants::TWO * (self.bp_medium_sum / self.tr_medium_sum);
                let third = self.bp_long_sum / self.tr_long_sum;
                return (first + second + third) * UltoscF64Constants::DIV;
            }
            F64Constants::ZERO
        }
        #[inline(always)]
        pub unsafe fn calc_unchecked(
            &mut self,
            high: &Simd<f64, N>,
            low: &Simd<f64, N>,
            close: &Simd<f64, N>,
            periods: (usize, usize),
        ) -> Simd<f64, N> {
            let (short_period, medium_period) = periods;
            let true_low = low.simd_min(self.prev_close);
            let true_high = high.simd_max(self.prev_close);
            let bp = close - true_low;
            let tr = true_high - true_low;

            let old = self.buffer.push_with_info_unchecked([bp, tr]);
            self.bp_long_sum += bp - old[0];
            self.tr_long_sum += tr - old[1];

            let [[bp_short, bp_medium], [tr_short, tr_medium]] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            self.bp_short_sum += bp - bp_short;
            self.bp_medium_sum += bp - bp_medium;
            self.tr_short_sum += tr - tr_short;
            self.tr_medium_sum += tr - tr_medium;

            self.prev_close = *close;

            let first = F64Constants::FOUR * (self.bp_short_sum / self.tr_short_sum);
            let second = F64Constants::TWO * (self.bp_medium_sum / self.tr_medium_sum);
            let third = self.bp_long_sum / self.tr_long_sum;

            (first + second + third) * UltoscF64Constants::DIV
        }
    }
}

pub mod options {
    use super::import::*;
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2>,
        periods: ([usize; N], [usize; N], [usize; N]),
        bp_short_sum: Simd<f64, N>,
        bp_medium_sum: Simd<f64, N>,
        bp_long_sum: Simd<f64, N>,

        tr_short_sum: Simd<f64, N>,
        tr_medium_sum: Simd<f64, N>,
        tr_long_sum: Simd<f64, N>,

        prev_close: f64,
    }

    impl<const N: usize> SimdState<N> {
        pub fn new(
            states: &mut [&mut State],
            periods: ([usize; N], [usize; N], [usize; N]),
        ) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");
            let mut main_buffer = 0;
            for i in 1..N {
                if states[main_buffer].buffer.capacity < states[i].buffer.capacity {
                    main_buffer = i;
                }
            }
            let buffer = states[main_buffer].buffer.clone();

            let mut bp_short_sum = [0.0; N];
            let mut bp_medium_sum = [0.0; N];
            let mut bp_long_sum = [0.0; N];

            let mut tr_short_sum = [0.0; N];
            let mut tr_medium_sum = [0.0; N];
            let mut tr_long_sum = [0.0; N];

            let prev_close = states[main_buffer].prev_close;

            for (i, state) in states.iter_mut().enumerate() {
                (bp_short_sum[i], bp_medium_sum[i], bp_long_sum[i]) =
                    (state.bp_sums_2x[0], state.bp_sums_2x[1], state.bp_long_sum);
                (tr_short_sum[i], tr_medium_sum[i], tr_long_sum[i]) =
                    (state.tr_sums_2x[0], state.tr_sums_2x[1], state.tr_long_sum);
            }

            Self {
                buffer,
                bp_short_sum: Simd::from_array(bp_short_sum),
                bp_medium_sum: Simd::from_array(bp_medium_sum),
                bp_long_sum: Simd::from_array(bp_long_sum),
                tr_short_sum: Simd::from_array(tr_short_sum),
                tr_medium_sum: Simd::from_array(tr_medium_sum),
                tr_long_sum: Simd::from_array(tr_long_sum),
                prev_close,
                periods,
            }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let vals: [[Vec<f64>; 2]; N] =
                std::array::from_fn(|i| self.buffer.to_ordered_by_period(self.periods.2[i]));

            let bp_short_sum = self.bp_short_sum.to_array();
            let bp_medium_sum = self.bp_medium_sum.to_array();
            let bp_long_sum = self.bp_long_sum.to_array();
            let tr_short_sum = self.tr_short_sum.to_array();
            let tr_medium_sum = self.tr_medium_sum.to_array();
            let tr_long_sum = self.tr_long_sum.to_array();

            for (i, vals) in vals.into_iter().enumerate() {
                states[i].buffer = {
                    let len = vals[0].len();
                    MultiBuffer {
                        vals,
                        index: 0,
                        prev_idx: len - 1,
                        count: len,
                        capacity: len,
                    }
                };

                (
                    states[i].bp_sums_2x[0],
                    states[i].bp_sums_2x[1],
                    states[i].bp_long_sum,
                ) = (bp_short_sum[i], bp_medium_sum[i], bp_long_sum[i]);
                (
                    states[i].tr_sums_2x[0],
                    states[i].tr_sums_2x[1],
                    states[i].tr_long_sum,
                ) = (tr_short_sum[i], tr_medium_sum[i], tr_long_sum[i]);
                states[i].prev_close = self.prev_close;
            }
        }

        /*#[inline(always)]
        pub fn calc(
            &mut self,
            high: f64,
            low: f64,
            close: f64,
        ) -> Simd<f64, N> {
            let (short_period, medium_period) = periods;

            let true_low = low.min(self.prev_close);
            let true_high = high.max(self.prev_close);
            let bp = close - true_low;
            let tr = true_high - true_low;

            if let Some(old) = self.buffer.push_with_info([bp, tr]) {
                self.bp_long_sum += bp - old[0];
                self.tr_long_sum += tr - old[1];
            } else {
                self.bp_long_sum += bp;
                self.tr_long_sum += tr;
            }
            let [[bp_short, bp_medium], [tr_short, tr_medium]] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            self.bp_short_sum += bp - bp_short;
            self.bp_medium_sum += bp - bp_medium;
            self.tr_short_sum += tr - tr_short;
            self.tr_medium_sum += tr - tr_medium;

            self.prev_close = close;

            if self.buffer.is_full() {
                let first = F64Constants::FOUR * (self.bp_short_sum / self.tr_short_sum);
                let second = F64Constants::TWO * (self.bp_medium_sum / self.tr_medium_sum);
                let third = self.bp_long_sum / self.tr_long_sum;
                return (first + second + third) * UltoscF64Constants::DIV;
            }
            F64Constants::ZERO
        }*/
        #[inline(always)]
        pub unsafe fn calc_unchecked(&mut self, high: f64, low: f64, close: f64) -> Simd<f64, N> {
            let (short_period, medium_period, long_period) = self.periods;
            let true_low = low.min(self.prev_close);
            let true_high = high.max(self.prev_close);
            let bp = close - true_low;
            let tr = true_high - true_low;

            let [bp_long_old, tr_long_old] = self
                .buffer
                .push_with_info_periods_unchecked([bp, tr], long_period);
            let bp = Simd::splat(bp);
            let tr = Simd::splat(tr);

            self.bp_long_sum += bp - Simd::from_array(bp_long_old);
            self.tr_long_sum += tr - Simd::from_array(tr_long_old);

            let [bp_medium_old, tr_medium_old] = self.buffer.get_by_periods(medium_period);
            self.bp_medium_sum += bp - Simd::from_array(bp_medium_old);
            self.tr_medium_sum += tr - Simd::from_array(tr_medium_old);

            let [bp_short_old, tr_short_old] = self.buffer.get_by_periods(short_period);
            self.bp_short_sum += bp - Simd::from_array(bp_short_old);
            self.tr_short_sum += tr - Simd::from_array(tr_short_old);

            self.prev_close = close;

            let first = F64Constants::FOUR * (self.bp_short_sum / self.tr_short_sum);
            let second = F64Constants::TWO * (self.bp_medium_sum / self.tr_medium_sum);
            let third = self.bp_long_sum / self.tr_long_sum;

            (first + second + third) * UltoscF64Constants::DIV
        }
    }
}
