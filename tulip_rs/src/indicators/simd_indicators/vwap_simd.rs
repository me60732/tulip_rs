#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vwap::indicator_by_assets;
use crate::indicators::{
    simd_indicators::typprice_simd::calc_simd as typprice_calc_simd, vwap::IndicatorState as State,
};
use std::simd::Simd;
pub use crate::indicator_types::{TSimdState, TState};
/// SIMD-parallel state for computing the Volume Weighted Average Price (VWAP) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    pub pv_sum: Simd<f64, N>,
    pub vol_sum: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
         sub: [],
         scalar: [pv_sum, vol_sum]
    );
}
impl<const N: usize> TState for SimdState<N>{
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, volume): Self::Inputs<'a>
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let tp = typprice_calc_simd(high, low, close);
        self.pv_sum += tp * volume;
        self.vol_sum += volume;
        (self.pv_sum / self.vol_sum, tp)
    }
}
