#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::mass::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::mass::indicator_by_options;
use std::ops::{Deref, DerefMut};
pub mod imports {
    pub(crate) use crate::indicators::mass::IndicatorState as State;
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::{calc_simd as ema_calc_simd, SimdState as EmaSimdState},
        simd_types::F64Constants,
    };
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}
use crate::types::Warm;
/// Asset-parallel SIMD computations for the Mass Index.
///
/// Provides [`SimdState`] for gathering `N` scalar states into SIMD lanes, advancing one
/// bar of Mass Index across all lanes simultaneously, and scattering results back to scalars.
pub mod asset {
    use super::imports::*;
    use super::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};

    /// SIMD-parallel state for computing the Mass Index across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub ema_state: EmaSimdState<N>,
        pub ema_signal: Simd<f64, N>,
        pub sum: Simd<f64, N>,
    }
    impl<const N: usize> Deref for SimdState<N> {
        type Target = EmaSimdState<N>;
        fn deref(&self) -> &Self::Target {
            &self.ema_state
        }
    }
    impl<const N: usize> DerefMut for SimdState<N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.ema_state
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;
        crate::simd_state_impl!(
             sub: [(ema_state: EmaSimdState<N>)],
             scalar: [ema_signal, sum],
             buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Simd<f64, N> {
            let hl_diff = (high - low).simd_max(F64Constants::EPSILON);

            let ema = self.ema_state.calc(hl_diff);
            self.ema_signal =
                ema_calc_simd(ema, self.ema_signal, self.multiplier, self.inv_multiplier);
            let mass = (ema / self.ema_signal).simd_max(F64Constants::ZERO);
            self.sum += mass - self.buffer.push_with_info(mass);

            self.sum
        }
    }
}

/// Option-parallel SIMD computations for the Mass Index with `N` different period settings on a single asset.
///
/// Shares the EMA/signal-EMA price state across all `N` lanes while maintaining per-lane
/// rolling sums over each lane's individual period.
pub mod option {
    use super::imports::*;
    use super::*;
    use crate::indicators::ema::{calc as ema_calc, State as EmaState};
    use crate::ring_buffer::single_buffer::generic_buffer::Buffer;

    /// State for computing the Mass Index with `N` different period options on a single asset.
    ///
    /// Each lane `i` has its own period and running sum, but the EMA/signal-EMA scalars are shared
    /// (computed from the same price series) and the ring buffer is sized to the largest period.
    pub struct SimdState<const N: usize> {
        pub buffer: Buffer<Warm>,
        pub sum: Simd<f64, N>,
        periods: [usize; N],
        pub ema_state: EmaState<Warm>,
        pub ema_signal: f64,
    }
    impl<const N: usize> Deref for SimdState<N> {
        type Target = EmaState<Warm>;
        fn deref(&self) -> &Self::Target {
            &self.ema_state
        }
    }
    impl<const N: usize> DerefMut for SimdState<N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.ema_state
        }
    }
    impl<'a, const N: usize> SimdState<N> {
        /// Initialises the option-mode state by borrowing `N` scalar [`State`] references.
        ///
        /// Picks the largest buffer (widest period) as the shared buffer and packs `sum` per lane.
        pub fn from_states(states: &'a mut [&mut State], periods: [usize; N]) -> Self {
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
                ema_state: states[0].ema_state.clone(),
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
                    state: std::marker::PhantomData::<Warm>,
                };
                state.sum = sum[i];
                state.ema_state = self.ema_state.clone();
                state.ema_signal = self.ema_signal;
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64);
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Self::Outputs {
            let hl_diff = (high - low).max(f64::EPSILON);
            let ema = self.ema_state.calc(hl_diff);
            self.ema_signal = ema_calc(ema, self.ema_signal, self.multiplier, self.inv_multiplier);
            let mass = (ema / self.ema_signal).max(0.0);
            self.sum += Simd::splat(mass)
                - Simd::from_array(self.buffer.push_with_info_periods(mass, self.periods));

            self.sum
        }
    }
}
