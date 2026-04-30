#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vhf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vhf::indicator_by_options;
use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, min_simd::SimdState as SimdMinState,
};
use crate::indicators::vhf::State;
use std::simd::{num::SimdFloat, Simd};

pub struct SimdState<const N: usize> {
    min_state: SimdMinState<N>,
    max_state: SimdMaxState<N>,
    sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut min_state = Vec::with_capacity(N);
        let mut max_state = Vec::with_capacity(N);
        let mut sum = [0.0; N];
        for (i, state) in states.iter_mut().enumerate() {
            min_state.push(&mut state.min_state);
            max_state.push(&mut state.max_state);
            sum[i] = state.sum;
        }
        let min_state = SimdMinState::new(&min_state);
        let max_state = SimdMaxState::new(&max_state);

        Self {
            min_state,
            max_state,
            sum: Simd::from_array(sum),
        }
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut max_refs = Vec::with_capacity(N);
        let mut min_refs = Vec::with_capacity(N);
        let sum = self.sum.as_array();

        for (i, state) in states.iter_mut().enumerate() {
            max_refs.push(&mut state.max_state);
            min_refs.push(&mut state.min_state);
            state.sum = sum[i];
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
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            real: [*const f64; N],
            look_back: usize,
            i: usize,
        ) -> Simd<f64, N>;
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const WINDOW_LANES: usize>(
            &mut self,
            values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            real: [*const f64; N],
            look_back: usize,
            i: usize,
        ) -> Simd<f64, N> {
            let (value, prev_real, old_real, drop_real) = values;
            self.sum += (value - prev_real).abs() - (old_real - drop_real).abs();

            let (min, _) = self
                .min_state
                .calc_unchecked_simd_w_current::<WINDOW_LANES>(real, i, look_back, value);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd_w_current::<WINDOW_LANES>(real, i, look_back, value);

            (max - min) / self.sum.simd_max(Simd::splat(f64::EPSILON))
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
            values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            real: [*const f64; N],
            look_back: Simd<usize, N>,
            i: Simd<usize, N>,
        ) -> Simd<f64, N>;
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            real: [*const f64; N],
            look_back: Simd<usize, N>,
            i: Simd<usize, N>,
        ) -> Simd<f64, N> {
            let (value, prev_real, old_real, drop_real) = values;
            self.sum += (value - prev_real).abs() - (old_real - drop_real).abs();

            let (min, _) = self
                .min_state
                .calc_unchecked_simd_w_current(real, i, look_back, value);
            let (max, _) = self
                .max_state
                .calc_unchecked_simd_w_current(real, i, look_back, value);

            (max - min) / self.sum.simd_max(Simd::splat(f64::EPSILON))
        }
    }
}
