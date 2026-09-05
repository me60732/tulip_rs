#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::ef::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::ef::indicator_by_options;
use crate::indicators::ef::State;
pub use crate::indicator_types::{TSimdState, TState};
use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd};
use crate::types::Warm;
pub struct SimdState<const N: usize> {
    pub sum: Simd<f64, N>,
    pub prev: Simd<f64, N>,
    pub drop: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [],
         scalar: [sum, prev, drop]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (value, last_value): (Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        self.sum += (value - self.prev).abs() - (last_value - self.drop).abs();
        self.prev = value;
        self.drop = last_value;
        let mask = self.sum.simd_ne(F64Constants::ZERO);
    
        mask.select(
            (value - last_value).abs() / self.sum, // When sum != 0.0
            F64Constants::ZERO,                // When sum == 0.0, return 0.0 (no efficiency)
        )
    }
}



