#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::stochrsi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::stochrsi::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::rsi_simd::SimdState as RsiSimdState;
use crate::indicators::stochrsi::State;
use crate::types::Warm;

use std::simd::{cmp::SimdPartialOrd, Select, Simd};
pub mod assets {
    use super::*;
    use crate::ring_buffer::multi_buffer::multi_mirror_buffer::MultiMirrorBuffer;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState, min_simd::assets::SimdState as MinSimdState,
    };
    /// SIMD-parallel state for computing the Stochastic RSI across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        pub buffer: MultiMirrorBuffer<N, f64, Warm>,
        pub rsi_state: RsiSimdState<N>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>), (rsi_state: RsiSimdState<N>)],
             scalar: [],
             buf: [(buffer: MultiMirrorBuffer<N, f64, Warm>, from_mirror_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc(
            &mut self,
            (real, period): Self::Inputs<'_>,
        ) -> Self::Outputs {
            let rsi = self.rsi_state.calc(real);
            self.buffer.push(rsi.to_array());

            let (min, _) = self
                .buffer
                .min(&mut self.min_state, rsi, period);
            let (max, _) = self
                .buffer
                .max(&mut self.max_state, rsi, period);

            let kdif = max - min;

            let kfast = kdif
                .simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (rsi - min) / kdif);

            (kfast, rsi)
        }
    }
}

pub mod options {
    use super::*;
    use crate::{
        indicator_types::TSimdState,
        ring_buffer::unsync_multi_buffer::unsync_mirror_buffer::UnsyncMirrorBuffer,
    };
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState, min_simd::options::SimdState as MinSimdState,
    };
    /// SIMD-parallel state for computing the Stochastic RSI across `N` option sets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to option set `i`.
    pub struct SimdState<const N: usize> {
        /// Rolling buffer of RSI values for each option lane.
        pub buffer: UnsyncMirrorBuffer<N, f64, Warm>,
        pub rsi_state: RsiSimdState<N>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [(rsi_state: RsiSimdState<N>), (min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [],
             buf: [(buffer: UnsyncMirrorBuffer<N, f64, Warm>, from_mirror_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc<'a>(&mut self, (real, period): Self::Inputs<'a>) -> (Simd<f64, N>, Simd<f64, N>) {
            let rsi = self.rsi_state.calc(real);
            self.buffer.push(rsi);

            let (min, _) = self.buffer.min(&mut self.min_state, rsi, period);
            let (max, _) = self.buffer.max(&mut self.max_state, rsi, period);

            let kdif = max - min;

            let kfast = kdif
                .simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (rsi - min) / kdif);

            (kfast, rsi)
        }
    }
}
