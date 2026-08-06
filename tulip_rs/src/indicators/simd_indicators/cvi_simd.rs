#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cvi::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cvi::indicator_by_options;
pub(crate) mod import {
    pub(crate) use crate::indicators::cvi::IndicatorState as State;
    pub(crate) use crate::indicators::simd_indicators::{
        ema_simd::SimdState as EmaSimdState, simd_types::F64Constants,
    };
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}
use crate::types::Warm;
pub mod assets {
    pub(crate) use super::import::*;
    use super::*;
    /// SIMD state alias for the CVI assets path — the state is a [`SimdBuffer`] of EMA values,
    /// one per asset lane, sized to the indicator's lookback period.
    pub(crate) use crate::ring_buffer::single_buffer::generic_buffer::SimdBuffer;
    use crate::ring_buffer::single_buffer::generic_buffer::SimdRingBuffer;

    pub struct SimdState<const N: usize> {
        buffer: SimdBuffer<N>,
        ema_state: EmaSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;
        crate::simd_state_impl!(
             sub: [(ema_state: EmaSimdState<N>)],
             scalar: [],
             buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Self::Outputs {
            let old_ema = self.buffer.front();
            let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
            let ema = self.ema_state.calc(hl_diff);
            self.buffer.push(ema);

            (ema - old_ema) / old_ema * F64Constants::HUNDRED
        }
    }
}

pub mod options {
    pub(crate) use super::import::*;
    use super::*;
    /// SIMD state alias for the CVI options path — per-lane ring buffers with potentially
    /// different periods stored in an `UnsyncBuffer`.
    pub(crate) use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;

    pub struct SimdState<const N: usize> {
        buffer: UnsyncBuffer<N, f64, Warm>,
        ema_state: EmaSimdState<N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;
        crate::simd_state_impl!(
             sub: [(ema_state: EmaSimdState<N>)],
             scalar: [],
             buf: [(buffer: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: f64,
            low: f64,
            multiplier: (Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N>;
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64);
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Self::Outputs {
            let hl_diff = Simd::splat((high - low).max(f64::EPSILON));

            let old_ema = self.buffer.front();

            let ema = self.ema_state.calc(hl_diff);
            self.buffer.push(ema);

            (ema - old_ema) / old_ema * F64Constants::HUNDRED
        }
    }
}
