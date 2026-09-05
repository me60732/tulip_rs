#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::stoch::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::stoch::indicator_by_options;
use crate::indicators::stoch::State;
use crate::types::Warm;
use std::simd::{num::SimdFloat, Simd};
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState, min_simd::assets::SimdState as MinSimdState,
    };

    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};

    /// SIMD-parallel state for computing the Stochastic Oscillator across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        pub prev_k: SimdBuffer<N>,
        pub prev_d: SimdBuffer<N>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
        pub k_sum: Simd<f64, N>,
        pub d_sum: Simd<f64, N>,
        pub k_multiplier: Simd<f64, N>,
        pub d_multiplier: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [k_sum, d_sum, k_multiplier, d_multiplier],
             buf: [(prev_k: SimdBuffer<N>, from_f64_buffers), (prev_d: SimdBuffer<N>, from_f64_buffers)]
        );
        crate::simd_state_write!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [k_sum, d_sum],
             buf: [(prev_k: SimdBuffer<N>, from_f64_buffers), (prev_d: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (
            [*const f64; N],
            [*const f64; N],
            Simd<f64, N>,
            usize,
            usize,
        );
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, close, i, look_back): Self::Inputs<'_>
        ) -> Self::Outputs {
            let kfast = {
                let (min, _) = self.min_state.calc((low, i, look_back));
                let (max, _) = self.max_state.calc((high, i, look_back));

                Simd::splat(100.0) * (close - min) / (max - min).simd_max(Simd::splat(f64::EPSILON))
            };

            let old_k = self.prev_k.push_with_info(kfast);
            self.k_sum += kfast - old_k;
            let k = self.k_sum * self.k_multiplier;
            let old_d = self.prev_d.push_with_info(k);
            self.d_sum += k - old_d;

            (k, self.d_sum * self.d_multiplier)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState, min_simd::options::SimdState as MinSimdState,
    };
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;

    /// SIMD-parallel state for computing the Stochastic Oscillator across `N` option sets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to option set `i`.
    pub struct SimdState<const N: usize> {
        pub prev_k: UnsyncBuffer<N, f64, Warm>,
        pub prev_d: UnsyncBuffer<N, f64, Warm>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
        pub k_sum: Simd<f64, N>,
        pub d_sum: Simd<f64, N>,
        pub k_multiplier: Simd<f64, N>,
        pub d_multiplier: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [k_sum, d_sum, k_multiplier, d_multiplier],
             buf: [(prev_k: UnsyncBuffer<N, f64>, from_f64_buffers), (prev_d: UnsyncBuffer<N, f64>, from_f64_buffers)]
        );
        crate::simd_state_write!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [k_sum, d_sum],
             buf: [(prev_k: UnsyncBuffer<N, f64, Warm>, from_f64_buffers), (prev_d: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (
            [*const f64; N],
            [*const f64; N],
            Simd<f64, N>,
            Simd<usize, N>,
            Simd<usize, N>,
        );
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc<'a>(
            &mut self,
            (high, low, close, i, look_back): Self::Inputs<'a>,
        ) -> Self::Outputs {
            let kfast = {
                let (min, _) = self.min_state.calc((low, i, look_back));
                let (max, _) = self.max_state.calc((high, i, look_back));

                Simd::splat(100.0) * (close - min) / (max - min).simd_max(Simd::splat(f64::EPSILON))
            };

            let (old_k, _) = self.prev_k.push_with_info(kfast);
            self.k_sum += kfast - old_k;
            let k = self.k_sum * self.k_multiplier;
            let (old_d, _) = self.prev_d.push_with_info(k);
            self.d_sum += k - old_d;

            (k, self.d_sum * self.d_multiplier)
        }
    }
}
