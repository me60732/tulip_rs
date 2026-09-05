#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::hma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::hma::indicator_by_options;

mod imports {
    pub(crate) use crate::indicators::hma::State;
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants, wma_simd::SimdState as WmaSimdState,
    };
    pub(crate) use std::simd::Simd;
}
pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;
pub mod assets {
    use super::imports::*;
    use super::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};
    /// SIMD-parallel state for the Hull Moving Average (HMA) indicator (per-asset variant),
    /// holding `N` lanes of per-asset state.
    pub struct SimdState<const N: usize> {
        pub prev_diff: SimdBuffer<N>,
        pub state1: WmaSimdState<N>,
        pub state2: WmaSimdState<N>,
        pub weighted_sumsqrt: Simd<f64, N>,
        weightssqrt: Simd<f64, N>,
        periodsqrt: Simd<f64, N>,
        pub sumsqrt: Simd<f64, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(state1: WmaSimdState<N>), (state2: WmaSimdState<N>)],
             scalar: [weighted_sumsqrt, sumsqrt, weightssqrt, periodsqrt],
             buf: [(prev_diff: SimdBuffer<N>, from_f64_buffers)]
        );
        crate::simd_state_write!(
             sub: [(state1: WmaSimdState<N>), (state2: WmaSimdState<N>)],
             scalar: [weighted_sumsqrt, sumsqrt],
             buf: [(prev_diff: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc<'a>(
            &mut self,
            (value, prev_value, prev_value2): Self::Inputs<'a>,
        ) -> Self::Outputs {
            let (wma, _) = self.state1.calc((value, prev_value));

            let (wma2, _) = self.state2.calc((value, prev_value2));
            let diff = F64Constants::TWO * wma2 - wma;
            self.weighted_sumsqrt += diff * self.periodsqrt;
            self.sumsqrt += diff;

            self.prev_diff.push(diff);

            let hma = self.weighted_sumsqrt / self.weightssqrt;
            self.weighted_sumsqrt -= self.sumsqrt;
            self.sumsqrt -= self.prev_diff.front();

            hma
        }
    }
}

pub mod options {
    use super::imports::*;
    use super::*;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;

    /// SIMD-parallel state for the Hull Moving Average (HMA) indicator (per-option variant),
    /// holding `N` lanes of per-option state.
    pub struct SimdState<const N: usize> {
        pub prev_diff: UnsyncBuffer<N, f64, Warm>,
        pub state1: WmaSimdState<N>,
        pub state2: WmaSimdState<N>,
        pub weighted_sumsqrt: Simd<f64, N>,
        weightssqrt: Simd<f64, N>,
        periodsqrt: Simd<f64, N>,
        pub sumsqrt: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(state1: WmaSimdState<N>), (state2: WmaSimdState<N>)],
             scalar: [weighted_sumsqrt, sumsqrt, weightssqrt, periodsqrt],
             buf: [(prev_diff: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
        crate::simd_state_write!(
             sub: [(state1: WmaSimdState<N>), (state2: WmaSimdState<N>)],
             scalar: [weighted_sumsqrt, sumsqrt],
             buf: [(prev_diff: UnsyncBuffer<N, f64>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc<'a>(
            &mut self,
            (value, prev_value, prev_value2): Self::Inputs<'a>,
        ) -> Self::Outputs {
            let (wma, _) = self.state1.calc((value, prev_value));

            let (wma2, _) = self.state2.calc((value, prev_value2));

            let diff = F64Constants::TWO * wma2 - wma;
            self.weighted_sumsqrt += diff * self.periodsqrt;
            self.sumsqrt += diff;

            self.prev_diff.push(diff);

            let hma = self.weighted_sumsqrt / self.weightssqrt;
            self.weighted_sumsqrt -= self.sumsqrt;
            self.sumsqrt -= self.prev_diff.front();

            hma
        }
    }
}
