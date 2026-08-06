#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trvi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trvi::indicator_by_options;

pub(crate) mod import {
    pub use crate::indicator_types::{TSimdState, TState};
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::SimdState as EmaSimdState, simd_types::F64Constants,
        tr_simd::SimdState as TrSimdState,
    };
    pub(crate) use crate::indicators::trvi::State;
    pub(crate) use crate::types::Warm;
    pub(crate) use std::simd::Simd;
}

pub mod assets {
    pub(crate) use super::import::*;
    /// SIMD state alias for the TRVI assets path — the state is a [`SimdBuffer`] of EMA values,
    /// one per asset lane, sized to the indicator's lookback period.
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::SimdBuffer;
    use crate::{
        indicator_types::TSimdState, ring_buffer::single_buffer::generic_buffer::SimdRingBuffer,
    };
    pub struct SimdState<const N: usize> {
        pub buffer: SimdBuffer<N>,
        pub ema_state: EmaSimdState<N>,
        pub tr_state: TrSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
            sub: [(ema_state: EmaSimdState<N>), (tr_state: TrSimdState<N>)],
            scalar: [],
            buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
            let old_ema = self.buffer.front();
            let tr = self.tr_state.calc(inputs);

            let ema = self.ema_state.calc(tr);
            self.buffer.push(ema);

            ((ema - old_ema) / old_ema * F64Constants::HUNDRED, tr, ema)
        }
    }
}

pub mod options {
    pub(crate) use super::import::*;
    use crate::indicators::tr::State as TrState;
    /// SIMD state alias for the TRVI options path — per-lane ring buffers with potentially
    /// different periods stored in an `UnsyncBuffer`.
    pub(crate) use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;
    pub struct SimdState<const N: usize> {
        pub buffer: UnsyncBuffer<N, f64, Warm>,
        pub ema_state: EmaSimdState<N>,
        pub tr_state: TrState,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        /// Gathers `N` scalar [`State`] references into a single `SimdState`,
        /// packing each field into a SIMD lane.
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");
            let tr_state = states[0].tr_state.clone();
            let mut buffer_refs = Vec::with_capacity(N);
            let mut ema_refs = Vec::with_capacity(N);
            for state in states.iter_mut() {
                buffer_refs.push(&state.buffer);
                ema_refs.push(&mut state.ema_state)
            }
            let buffer = UnsyncBuffer::<N, f64, Warm>::from_f64_buffers(buffer_refs);
            let ema_state = EmaSimdState::from_states(&mut ema_refs);
            Self {
                buffer,
                ema_state,
                tr_state,
            }
        }

        /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in
        /// place, avoiding allocation compared to a `to_states` conversion.
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let mut ema_refs = Vec::with_capacity(N);
            for (state, buffer) in states.iter_mut().zip(buffers.into_iter()) {
                state.buffer = buffer;
                state.tr_state = self.tr_state.clone();
                ema_refs.push(&mut state.ema_state)
            }
            self.ema_state.write_states(&mut ema_refs);
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64, f64);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
            let old_ema = self.buffer.front();
            let tr = Simd::splat(self.tr_state.calc(inputs));

            let ema = self.ema_state.calc(tr);
            self.buffer.push(ema);

            ((ema - old_ema) / old_ema * F64Constants::HUNDRED, tr, ema)
        }
    }
}
