#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::emv::indicator_by_assets;

use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, simd_types::F64Constants,
};
use std::simd::{num::SimdFloat, *};
pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::emv::State;
use crate::types::Warm;
pub struct SimdState<const N: usize> {
    prev_medprice: Simd<f64, N>
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [],
         scalar: [prev_medprice]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, volume): Self::Inputs<'a>
    ) -> Self::Outputs {
        let medprice = calc_medprice_simd(high, low);
        let distance_moved = medprice - self.prev_medprice;
        self.prev_medprice = medprice;
        let hl_diff = (high - low).simd_max(F64Constants::EPSILON);
        let volume_safe = volume.simd_max(F64Constants::EPSILON);
    
        (distance_moved * F64Constants::TEN_THOUSAND * hl_diff / volume_safe, medprice)
    }
}

