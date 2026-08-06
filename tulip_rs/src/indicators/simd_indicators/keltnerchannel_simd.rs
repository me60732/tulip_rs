#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::keltnerchannel::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::keltnerchannel::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::keltnerchannel::State;
use crate::indicators::simd_indicators::{
    atr_simd::SimdState as AtrSimdState, ema_simd::SimdState as EmaSimdState,
};
use crate::types::Warm;

use std::simd::Simd;
/// SIMD-parallel state for computing the Keltner Channel across `N` assets or option-set lanes.
///
/// Holds a Wilder ATR state and an EMA value for each lane packed into SIMD vectors.
pub struct SimdState<const N: usize> {
    pub atr_state: AtrSimdState<N>,
    pub ema_state: EmaSimdState<N>,
    pub step: Simd<f64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;

    crate::simd_state_from_state!(
        sub: [(atr_state: AtrSimdState<N>), (ema_state: EmaSimdState<N>)],
        scalar: [step]
    );
    crate::simd_state_write!(
         sub: [(atr_state: AtrSimdState<N>), (ema_state: EmaSimdState<N>)],
         scalar: []
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    );

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let (atr, tr) = self.atr_state.calc((high, low, close));
        let ema = self.ema_state.calc(close);

        let per = atr * self.step;
        let upper = ema + per;
        let lower = ema - per;
        //let upper = atr.mul_add(step, self.ema);
        //let lower = atr.mul_add(-step, self.ema);

        (lower, ema, upper, atr, tr)
    }
}
