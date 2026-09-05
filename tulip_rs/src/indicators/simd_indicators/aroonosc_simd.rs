/// Re-uses [`aroon_simd::SimdState`] as the state for the Aroon Oscillator since both
/// indicators track the same rolling min/max windows.

#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::aroonosc::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::aroonosc::State;
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::aroonosc::indicator_by_options;
pub use crate::types::Warm;
use std::ops::{Deref, DerefMut};
use std::simd::Simd;
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::aroon_simd::assets::SimdState as AroonSimdState;

    #[repr(transparent)]
    pub struct SimdState<const N: usize>(pub AroonSimdState<N>);
    impl<const N: usize> Deref for SimdState<N> {
        type Target = AroonSimdState<N>;
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

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            Self(AroonSimdState::from_states(&mut inner))
        }
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            self.0.write_states(&mut inner)
        }
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], usize, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc(self: &mut Self, inputs: Self::Inputs<'_>) -> Self::Outputs {
            let (aroon_down, aroon_up) = self.0.calc(inputs);

            (aroon_up - aroon_down, aroon_down, aroon_up)
        }
    }
}
pub mod options {
    use super::*;
    pub(crate) use crate::indicators::simd_indicators::aroon_simd::options::SimdState as AroonSimdState;

    #[repr(transparent)]
    pub struct SimdState<const N: usize>(pub AroonSimdState<N>);
    impl<const N: usize> Deref for SimdState<N> {
        type Target = AroonSimdState<N>;
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

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            Self(AroonSimdState::from_states(&mut inner))
        }
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            let mut inner: Vec<&mut _> = states.iter_mut().map(|s| &mut s.0).collect();
            self.0.write_states(&mut inner)
        }
    }
    
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = ([*const f64; N], [*const f64; N], Simd<usize, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        
        #[inline(always)]
        fn calc(
            self: &mut Self,
            inputs: Self::Inputs<'_>
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (aroon_down, aroon_up) = self.0.calc(inputs);

            (aroon_up - aroon_down, aroon_down, aroon_up)
        }
    }
}
