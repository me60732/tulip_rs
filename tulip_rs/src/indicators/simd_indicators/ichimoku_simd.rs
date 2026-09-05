use crate::indicators::ichimoku::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::ichimoku::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::ichimoku::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;
use std::simd::Simd;

/// SIMD-parallel state for computing the Ichimoku Cloud across `N` assets or option-sets simultaneously.
///
/// Wraps six min/max ring-buffer SIMD states: one pair each for the short (Tenkan-sen),
/// medium (Kijun-sen), and ultra-long (Senkou Span B) lookback windows.

pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState,
        min_simd::assets::SimdState as MinSimdState,
    };

    pub struct SimdState<const N: usize> {
        short_min_state: MinSimdState<N>,
        short_max_state: MaxSimdState<N>,
        medium_min_state: MinSimdState<N>,
        medium_max_state: MaxSimdState<N>,
        long_min_state: MinSimdState<N>,
        long_max_state: MaxSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [(short_min_state: MinSimdState<N>), (medium_min_state: MinSimdState<N>), (long_min_state: MinSimdState<N>), (short_max_state: MaxSimdState<N>), (medium_max_state: MaxSimdState<N>), (long_max_state: MaxSimdState<N>)],
             scalar: []
        );
    }
    /// SIMD computation trait for the Ichimoku Cloud, operating on `N` asset lanes simultaneously.
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], usize, usize, usize, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
            self.calc_chuncked::<1, 4, 4>(inputs)
        }
    }
    impl<const N: usize> SimdState<N> {
        #[inline(always)]
        fn calc_chuncked<const CS: usize, const CM: usize, const CL: usize>(
            &mut self,
            (high, low, i, short_look_back, long_look_back, ultra_look_back): (
                [*const f64; N],
                [*const f64; N],
                usize,
                usize,
                usize,
                usize,
            ),
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (short_min, _) =
                self.short_min_state
                    .calc_chuncked::<CS>((low, i, short_look_back));
            let (short_max, _) =
                self.short_max_state
                    .calc_chuncked::<CS>((high, i, short_look_back));
            let (medium_min, _) =
                self.medium_min_state
                    .calc_chuncked::<CM>((low, i, long_look_back));
            let (medium_max, _) =
                self.medium_max_state
                    .calc_chuncked::<CM>((high, i, long_look_back));
            let (long_min, _) = self
                .long_min_state
                .calc_chuncked::<CL>((low, i, ultra_look_back));
            let (long_max, _) = self
                .long_max_state
                .calc_chuncked::<CL>((high, i, ultra_look_back));

            let half = Simd::splat(0.5_f64);
            let conversion = half * (short_min + short_max);
            let base = half * (medium_min + medium_max);
            let span_a = half * (conversion + base);
            let span_b = half * (long_min + long_max);

            (conversion, base, span_a, span_b)
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
        short_min_state: MinSimdState<N>,
        short_max_state: MaxSimdState<N>,
        medium_min_state: MinSimdState<N>,
        medium_max_state: MaxSimdState<N>,
        long_min_state: MinSimdState<N>,
        long_max_state: MaxSimdState<N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
             sub: [(short_min_state: MinSimdState<N>), (medium_min_state: MinSimdState<N>), (long_min_state: MinSimdState<N>), (short_max_state: MaxSimdState<N>), (medium_max_state: MaxSimdState<N>), (long_max_state: MaxSimdState<N>)],
             scalar: []
        );
    }
    

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<usize, N>, Simd<usize, N>, Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, i, short_look_back, long_look_back, ultra_look_back): Self::Inputs<'_>
        ) -> Self::Outputs {
            let (short_min, _) = self
                .short_min_state
                .calc((low, i, short_look_back));
            let (short_max, _) = self
                .short_max_state
                .calc((high, i, short_look_back));
            let (medium_min, _) = self
                .medium_min_state
                .calc((low, i, long_look_back));
            let (medium_max, _) =
                self.medium_max_state
                    .calc((high, i, long_look_back));
            let (long_min, _) = self
                .long_min_state
                .calc((low, i, ultra_look_back));
            let (long_max, _) = self
                .long_max_state
                .calc((high, i, ultra_look_back));

            let half = Simd::splat(0.5_f64);
            let conversion = half * (short_min + short_max);
            let base = half * (medium_min + medium_max);
            let span_a = half * (conversion + base);
            let span_b = half * (long_min + long_max);

            (conversion, base, span_a, span_b)
        }
    }
}
