use std::simd::{num::SimdFloat, Simd};

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::cmo::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::cmo::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::cmo::indicator_by_options;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use crate::types::Warm;
//use crate::math_simd::fast_max;
/// SIMD-parallel state for computing the Chande Momentum Oscillator (CMO) across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub up_sum: Simd<f64, N>,
    pub down_sum: Simd<f64, N>,
    pub prev: Simd<f64, N>,
    pub drop_real: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [],
         scalar: [up_sum, down_sum, prev, drop_real]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (cur_real, old_real): Self::Inputs<'a>,
    ) -> Simd<f64, N> {
        let (old_up, old_down) = up_down_simd(old_real, self.drop_real);
        self.drop_real = old_real;
        let (up, down) = up_down_simd(cur_real, self.prev);
        self.prev = cur_real;
        self.up_sum += up - old_up;
        self.down_sum += down - old_down;

        F64Constants::HUNDRED * (self.up_sum - self.down_sum) / (self.up_sum + self.down_sum)
    }
}

/// Splits a price change into its up and down components across all `N` lanes.
///
/// `up = max(value - prev_value, 0)` and `down = max(prev_value - value, 0)`.
#[inline(always)]
pub fn up_down_simd<const N: usize>(
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let diff = value - prev_value;
    (
        diff.simd_max(F64Constants::ZERO),
        (-diff).simd_max(F64Constants::ZERO),
    )
}
