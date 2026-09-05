#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::tr::indicator_by_assets;
use std::simd::{num::SimdFloat, Simd};
pub use crate::indicator_types::{TState, TSimdState};
pub use crate::indicators::tr::State;
pub struct SimdState<const N: usize> {
    pub prev_close: Simd<f64, N>
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc(
        &mut self,
        (high, low, close): (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>)
    ) -> Simd<f64, N> {
        let true_low = low.simd_min(self.prev_close);
        let true_high = high.simd_max(self.prev_close);
        self.prev_close = close;
        true_high - true_low
    }
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_impl!(
        sub: [],
        scalar: [prev_close]
    );
}