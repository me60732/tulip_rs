#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::mfi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::mfi::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
pub(crate) mod imports {
    pub(crate) use crate::indicators::mfi::IndicatorState as State;
    pub(crate) use crate::indicators::simd_indicators::simd_types::F64Constants;
    pub(crate) use crate::ring_buffer::multi_buffer::multi_buffer::{
        MultiBuffer, SimdRingBuffer,
    };
    pub(crate) use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd};
}
pub use crate::types::Warm;
pub mod assets {
    use super::imports::*;
    use super::*;
    use crate::indicators::simd_indicators::typprice_simd::calc_simd as typprice_calc_simd;
    /// SIMD-parallel state for computing the Money Flow Index (MFI) across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2, Simd<f64, N>, Warm>,
        /// Most recent typical price `(high + low + close) / 3` per asset lane.
        pub typprice: Simd<f64, N>,
        /// Running sum of positive money flow (typical price * volume when price rises) per lane.
        pub pos_sum: Simd<f64, N>,
        /// Running sum of negative money flow (typical price * volume when price falls) per lane.
        pub neg_sum: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;
        /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
        fn from_states(states: &mut [&mut State]) -> Self {
            let buffer_refs: [&MultiBuffer<2, f64, Warm>; N] =
                core::array::from_fn(|i| &states[i].buffer);
            let buffer = <MultiBuffer<2, Simd<f64, N>, Warm> as SimdRingBuffer<2, N>>::from_f64_buffers(
                buffer_refs,
            );

            let mut typprice = [0.0; N];
            let mut pos_sum = [0.0; N];
            let mut neg_sum = [0.0; N];

            for (i, state) in states.iter().enumerate() {
                typprice[i] = state.typprice;
                pos_sum[i] = state.pos_sum;
                neg_sum[i] = state.neg_sum;
            }

            Self {
                buffer,
                typprice: Simd::from_array(typprice),
                pos_sum: Simd::from_array(pos_sum),
                neg_sum: Simd::from_array(neg_sum),
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
        fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffer = self.buffer.to_f64_buffers();
            let typprice = self.typprice.to_array();
            let pos_sum = self.pos_sum.to_array();
            let neg_sum = self.neg_sum.to_array();

            for (i, buffer) in buffer.into_iter().enumerate() {
                states[i].buffer = buffer;
                states[i].typprice = typprice[i];
                states[i].pos_sum = pos_sum[i];
                states[i].neg_sum = neg_sum[i];
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;
        /// Computes one MFI step across `N` asset lanes using SIMD parallelism.
        ///
        /// Classifies each bar's money flow as positive or negative based on the sign of
        /// `typical_price_change`, maintains rolling sums over the window, and returns
        /// `pos_sum / (pos_sum + neg_sum) * 100` (clamped to avoid division by zero).
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
            let prev_typprice = self.typprice;
            self.typprice = typprice_calc_simd(high, low, close);

            let price_change = self.typprice - prev_typprice;
            let money_flow = self.typprice * volume;

            let pos_mask = price_change.simd_gt(F64Constants::ZERO);
            let neg_mask = price_change.simd_lt(F64Constants::ZERO);

            let pos_flow = pos_mask.select(money_flow, F64Constants::ZERO);
            let neg_flow = neg_mask.select(money_flow, F64Constants::ZERO);

            let old = self.buffer.push_with_info([pos_flow, neg_flow]);
            self.pos_sum += pos_flow - old[0];
            self.neg_sum += neg_flow - old[1];

            self.pos_sum / (self.pos_sum + self.neg_sum).simd_max(F64Constants::EPSILON)
                * F64Constants::HUNDRED
        }
    }
}

pub mod options {
    use super::imports::*;
    use super::*;
    use crate::indicators::typprice::calc as typprice_calc;
    /// State for computing the MFI with `N` different period options on a single asset.
    ///
    /// Each lane `i` has its own period and positive/negative sums, but the typical price and
    /// the ring buffer are shared (sized to the widest period).
    pub struct SimdState<const N: usize> {
        buffer: MultiBuffer<2, f64, Warm>,
        /// Shared most recent typical price (scalar, same series for all lanes).
        pub typprice: f64,
        /// Per-lane running positive money flow sum.
        pub pos_sum: Simd<f64, N>,
        /// Per-lane running negative money flow sum.
        pub neg_sum: Simd<f64, N>,
        periods: [usize; N],
    }

    impl<const N: usize> SimdState<N> {
        /// Initialises the option-mode MFI state by borrowing `N` scalar [`State`] references.
        ///
        /// Uses the widest buffer and packs per-lane positive/negative sums.
        pub fn from_states(states: &mut [&mut State], periods: [usize; N]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut main_buffer = 0;
            for i in 1..N {
                if states[main_buffer].buffer.capacity < states[i].buffer.capacity {
                    main_buffer = i;
                }
            }
            let buffer = states[main_buffer].buffer.clone();

            let mut pos_sum = [0.0; N];
            let mut neg_sum = [0.0; N];

            for (i, state) in states.iter().enumerate() {
                pos_sum[i] = state.pos_sum;
                neg_sum[i] = state.neg_sum;
            }

            Self {
                buffer,
                typprice: states[main_buffer].typprice,
                pos_sum: Simd::from_array(pos_sum),
                neg_sum: Simd::from_array(neg_sum),
                periods,
            }
        }

        /// Writes the option-mode SIMD state back into `N` existing mutable scalar [`State`] references.
        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let vals: [[Vec<f64>; 2]; N] =
                std::array::from_fn(|i| self.buffer.to_ordered_by_period(self.periods[i]));
            let typprice = self.typprice;
            let pos_sum = self.pos_sum.to_array();
            let neg_sum = self.neg_sum.to_array();

            for (i, vals) in vals.into_iter().enumerate() {
                states[i].buffer = {
                    let len = vals[0].len();
                    MultiBuffer {
                        vals,
                        index: 0,
                        prev_idx: len - 1,
                        capacity: len,
                        count: len,
                        state: std::marker::PhantomData,
                    }
                };
                states[i].typprice = typprice;
                states[i].pos_sum = pos_sum[i];
                states[i].neg_sum = neg_sum[i];
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64, f64, f64);
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
            let prev_typprice = self.typprice;
            self.typprice = typprice_calc(high, low, close);

            let price_change = self.typprice - prev_typprice;
            let money_flow = self.typprice * volume;

            let (pos_flow, neg_flow) = if price_change > 0.0 {
                (money_flow, 0.0)
            } else if price_change < 0.0 {
                (0.0, money_flow)
            } else {
                (0.0, 0.0)
            };

            let [pos_flow_old, neg_flow_old] = self
                .buffer
                .push_with_info_periods([pos_flow, neg_flow], self.periods);
            self.pos_sum += Simd::splat(pos_flow) - Simd::from_array(pos_flow_old);
            self.neg_sum += Simd::splat(neg_flow) - Simd::from_array(neg_flow_old);

            self.pos_sum / (self.pos_sum + self.neg_sum).simd_max(F64Constants::EPSILON)
                * F64Constants::HUNDRED
        }
    }
}
