#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::willr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::willr::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};

use crate::indicators::willr::State;
use std::simd::{cmp::SimdPartialOrd, Select, Simd};
/// SIMD-parallel state for the Williams %R indicator, holding `N` lanes of per-asset state.
use crate::types::Warm;
pub mod assets {
    //! Per-asset road SIMD helpers for the Williams %R indicator.
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState, min_simd::assets::SimdState as MinSimdState,
    };

    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
                 sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
                 scalar: []
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<f64, N>, usize, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, close, i, look_back): Self::Inputs<'_>
        ) -> Self::Outputs {
            // Update the minimum and maximum for the rolling window.
            let (min, _) = self
                .min_state
                .calc((low, i, look_back));
            let (max, _) = self
                .max_state
                .calc((high, i, look_back));

            let mm = max - min;
            (
                mm.simd_lt(Simd::splat(f64::EPSILON))
                    .select(Simd::splat(0.0), Simd::splat(100.0) * (max - close) / mm),
                min,
                max,
            )
        }
    }
}

pub mod options {
    //! Per-option road SIMD helpers for the Williams %R indicator.
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState, min_simd::options::SimdState as MinSimdState,
    };
    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
                 sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
                 scalar: []
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<f64, N>, Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, close, i, look_back): Self::Inputs<'_>,
        ) -> Self::Outputs {
            // Update the minimum and maximum for the rolling window.
            let (min, _) = self.min_state.calc((low, i, look_back));
            let (max, _) = self.max_state.calc((high, i, look_back));

            let mm = max - min;
            (
                mm.simd_lt(Simd::splat(f64::EPSILON))
                    .select(Simd::splat(0.0), Simd::splat(100.0) * (max - close) / mm),
                min,
                max,
            )
        }
    }
}
