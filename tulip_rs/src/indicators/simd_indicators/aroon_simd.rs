use crate::indicators::aroon::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::aroon::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::aroon::indicator_by_options;

use std::simd::{num::SimdUint, Simd};
pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;

pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState, min_simd::assets::SimdState as MinSimdState,
    };

    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        multiplier: Simd<f64, N>
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [multiplier]
        );
        crate::simd_state_write!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: []
        );
    }
    
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], usize, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, i, period): Self::Inputs<'_>,
        ) -> Self::Outputs {
            let period_simd = Simd::splat(period);

            let (_, min_trail) = self
                .min_state
                .calc((low, i, period));
            let (_, max_trail) = self
                .max_state
                .calc((high, i, period));

            let aroon_up = (period_simd - max_trail).cast() * self.multiplier;
            let aroon_down = (period_simd - min_trail).cast() * self.multiplier;

            (aroon_down, aroon_up)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState, min_simd::options::SimdState as MinSimdState,
    };

    pub struct SimdState<const N: usize> {
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        multiplier: Simd<f64, N>
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_from_state!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [multiplier]
        );
        crate::simd_state_write!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: []
        );
    }
    
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);
        
        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, i, period): Self::Inputs<'_>,
        ) -> Self::Outputs {
            let (_, min_trail) = self.min_state.calc((low, i, period));
            let (_, max_trail) = self.max_state.calc((high, i, period));

            let aroon_up = (period - max_trail).cast() * self.multiplier;
            let aroon_down = (period - min_trail).cast() * self.multiplier;

            (aroon_down, aroon_up)
        }
    }
}
