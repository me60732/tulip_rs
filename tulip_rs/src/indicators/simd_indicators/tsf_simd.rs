pub(crate) use crate::indicators::simd_indicators::linreg_simd::SimdState as LinregSimdState;

#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::tsf::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::tsf::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};
use crate::indicators::tsf::State;
use crate::types::Warm;

pub struct SimdState<const N: usize> {
    linreg_state: LinregSimdState<N>,
    sum_x: Simd<f64, N>,
    per: Simd<f64, N>, 
    inv_n: Simd<f64, N>
    
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
        sub: [(linreg_state: LinregSimdState<N>)],
        scalar: [sum_x, per, inv_n]
    );
    crate::simd_state_write!(
        sub: [(linreg_state: LinregSimdState<N>)],
        scalar: []
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (prev_value, value): Self::Inputs<'a>,
    ) -> Self::Outputs {
        let (linreg, slope, intercept) = self.linreg_state.calc((prev_value, value, (self.sum_x, self.per, self.inv_n)));
        //let tsf = intercept + slope * (period + F64Constants::ONE);
        let tsf = slope.mul_add(self.linreg_state.n + F64Constants::ONE, intercept);
        (tsf, linreg, slope, intercept)
    }
}
impl<const N: usize> SimdState<N> {
    #[inline(always)]
    pub fn partial_calc(
        &mut self,
        (prev_value, value): (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (_, slope, intercept) = self.linreg_state.calc((prev_value, value, (self.sum_x, self.per, self.inv_n)));
        //let tsf = intercept + slope * (period + F64Constants::ONE);
        let tsf = slope.mul_add(self.linreg_state.n + F64Constants::ONE, intercept);
        (tsf, slope, intercept)
    }
}