#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::willr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::willr::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, min_simd::SimdState as SimdMinState,
};
use crate::indicators::willr::State;
use std::simd::{cmp::SimdPartialOrd, Select, Simd};

pub struct SimdState<const N: usize> {
    min_state: SimdMinState<N>,
    max_state: SimdMaxState<N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut min_state = Vec::with_capacity(N);
        let mut max_state = Vec::with_capacity(N);

        for state in states.iter_mut() {
            min_state.push(&mut state.min_state);
            max_state.push(&mut state.max_state);
        }
        let min_state = SimdMinState::new(&min_state);
        let max_state = SimdMaxState::new(&max_state);

        Self {
            min_state,
            max_state,
        }
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut max_refs = Vec::with_capacity(N);
        let mut min_refs = Vec::with_capacity(N);

        for state in states.iter_mut() {
            max_refs.push(&mut state.max_state);
            min_refs.push(&mut state.min_state);
        }
        self.max_state.write_states(&mut max_refs);
        self.min_state.write_states(&mut min_refs);
    }
}
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::Calc as CalcMax, min_simd::assets::Calc as CalcMin,
    };

    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd<const CHUNK_SIZE: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: usize,
            look_back: usize,
        ) -> Simd<f64, N>;
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const CHUNK_SIZE: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: usize,
            look_back: usize,
        ) -> Simd<f64, N> {
            // Update the minimum and maximum for the rolling window.
            let (min, _) = self
                .min_state
                .calc_unchecked_simd::<CHUNK_SIZE>(low, i, look_back);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd::<CHUNK_SIZE>(high, i, look_back);

            let mm = max - min;
            mm.simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (max - close) / mm)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> Simd<f64, N>;
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            close: Simd<f64, N>,
            i: Simd<usize, N>,
            look_back: Simd<usize, N>,
        ) -> Simd<f64, N> {
            // Update the minimum and maximum for the rolling window.
            let (min, _) = self.min_state.calc_unchecked_simd(low, i, look_back);
            let (max, _) = self.max_state.calc_unchecked_simd(high, i, look_back);

            let mm = max - min;
            mm.simd_lt(Simd::splat(f64::EPSILON))
                .select(Simd::splat(0.0), Simd::splat(100.0) * (max - close) / mm)
        }
    }
}
