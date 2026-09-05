use crate::indicators::chandelierexit::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::chandelierexit::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::chandelierexit::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::atr_simd::SimdState as AtrSimdState;
use crate::types::Warm;
use std::simd::{Simd, StdFloat};
/// SIMD-parallel state for computing the Chandelier Exit indicator across `N` assets or option-sets simultaneously.
/// Wraps dedicated min/max ring-buffer SIMD states and a Wilder ATR state, one per lane.

pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState,
        min_simd::assets::SimdState as MinSimdState,
    };

    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        atr_state: AtrSimdState<N>,
        step: Simd<f64, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>), (atr_state: AtrSimdState<N>)],
            scalar: [step]
        );
        crate::simd_state_write!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>), (atr_state: AtrSimdState<N>)],
            scalar: []
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<f64, N>, usize, usize);
        type Outputs = (
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
        );

        #[inline(always)]
        fn calc(
            &mut self,
            (high_ptrs, low_ptrs, close, i, look_back): Self::Inputs<'_>,
        ) -> Self::Outputs {
            let (min, _) = self.min_state.calc((low_ptrs, i, look_back));
            let (max, _) = self.max_state.calc((high_ptrs, i, look_back));

            let (high, low) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (atr, tr) = self.atr_state.calc((high, low, close));

            let long = atr.mul_add(-self.step, max);
            let short = atr.mul_add(self.step, min);

            (long, short, atr, tr, min, max)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState,
        min_simd::options::SimdState as MinSimdState,
    };
    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        atr_state: AtrSimdState<N>,
        step: Simd<f64, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>), (atr_state: AtrSimdState<N>)],
            scalar: [step]
        );
        crate::simd_state_write!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>), (atr_state: AtrSimdState<N>)],
            scalar: []
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
        type Outputs = (
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
        );

        #[inline(always)]
        fn calc(
            &mut self,
            (high_ptrs, low_ptrs, close, i, look_back): Self::Inputs<'_>,
        ) -> Self::Outputs {
            let (min, _) = self.min_state.calc((low_ptrs, i, look_back));
            let (max, _) = self.max_state.calc((high_ptrs, i, look_back));

            let (high, low) = crate::extract_simd_inputs_at_index_array!(i.as_array(), N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (atr, tr) = self.atr_state.calc((high, low, close));

            let long = atr.mul_add(-self.step, max);
            let short = atr.mul_add(self.step, min);

            (long, short, atr, tr, min, max)
        }
    }
}
