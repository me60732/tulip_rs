#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::md::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::md::indicator_by_options;
use crate::types::Warm;
pub mod imports {
    pub(crate) use crate::indicators::{
        md::State,
        simd_indicators::{simd_types::F64Constants, sma_simd::SimdState as SmaSimdState},
    };
    pub use std::ops::{Deref, DerefMut};
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}
pub mod assets {
    use super::imports::*;
    use super::*;
    #[repr(transparent)]
    pub struct SimdState<const N: usize>(pub SmaSimdState<N>);
    impl<const N: usize> Deref for SimdState<N> {
        type Target = SmaSimdState<N>;
        #[inline(always)]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const N: usize> DerefMut for SimdState<N> {
        #[inline(always)]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, &'a [Simd<f64, N>]);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc<'a>(
            &mut self,
            (value, prev_value, slice): Self::Inputs<'a>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let sma = self.0.calc((value, prev_value));

            let md = self.calc_md_simd(slice, sma);
            (md, sma)
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            Self(SmaSimdState::from_states(&mut inner))
        }
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            self.0.write_states(&mut inner)
        }
    }
    impl<const N: usize> SimdState<N> {
        #[inline(always)]
        pub fn calc_md_simd(&self, slice: &[Simd<f64, N>], sma: Simd<f64, N>) -> Simd<f64, N> {
            (slice.iter().map(|&x| (x - sma).abs()).sum::<Simd<f64, N>>() * self.multiplier)
                .simd_max(F64Constants::EPSILON)
        }
    }
}
pub mod options {
    use super::imports::*;
    use super::*;
    use crate::indicators::md::calc_md_simd;
    #[repr(transparent)]
    pub struct SimdState<const N: usize>(pub SmaSimdState<N>);
    impl<const N: usize> Deref for SimdState<N> {
        type Target = SmaSimdState<N>;
        #[inline(always)]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const N: usize> DerefMut for SimdState<N> {
        #[inline(always)]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (
            Simd<f64, N>,
            Simd<f64, N>,
            [*const f64; N],
            [usize; N],
            [usize; N],
        );
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc<'a>(
            &mut self,
            (value, prev_value, real, periods, i): Self::Inputs<'a>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let sma = self.0.calc((value, prev_value));
            let mut md = [0.0; N];
            let sma_ref = sma.as_array();
            let multiplier_ref = self.multiplier.as_array();

            //let take = (i + Simd::splat(1)) - start;
            for (lane, (&i, &period)) in i.iter().zip(periods.iter()).enumerate() {
                let start = i + 1 - period;
                let slice = unsafe { std::slice::from_raw_parts(real[lane].add(start), period) };
                md[lane] = calc_md_simd::<4>(slice, sma_ref[lane], multiplier_ref[lane]);
            }
            (Simd::from_array(md), sma)
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            Self(SmaSimdState::from_states(&mut inner))
        }
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            self.0.write_states(&mut inner)
        }
    }
}
