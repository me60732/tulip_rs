#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::supertrend::indicator_by_assets;
use crate::indicators::simd_indicators::{
    atr_simd::SimdState as AtrSimdState, medprice_simd::calc_simd as medprice_calc_simd,
};
use crate::indicators::supertrend::State;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::supertrend::indicator_by_options;
use crate::types::Warm;
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Mask, Select, Simd};
/// SIMD-parallel state for computing the SuperTrend indicator across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    pub atr_state: AtrSimdState<N>,
    pub prev_st: Simd<f64, N>,
    pub prev_ub: Simd<f64, N>,
    pub prev_lb: Simd<f64, N>,
    pub step: Simd<f64, N>,
    pub trend: Mask<i64, N>,
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(atr_state: AtrSimdState<N>)],
         scalar: [prev_st, prev_ub, prev_lb, step],
         buf: [],
         mask: [trend]
    );
    crate::simd_state_write!(
         sub: [(atr_state: AtrSimdState<N>)],
         scalar: [prev_st, prev_ub, prev_lb],
         buf: [],
         mask: [trend]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    /// Advances all `N` lanes by one bar, returning `(supertrend, atr, tr, medprice)`.
    ///
    /// Delegates ATR computation to `atr_state.calc_simd`, scales the result by `step` to
    /// obtain the band half-width, then updates per-lane trend flags and band ratchets.
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let (atr, tr) = self.atr_state.calc((high, low, close));
        let step = self.step * atr;
        let (st, medprice) = self.calc_st(high, low, close, step);
        (st, atr, tr, medprice)
    }
}
impl<const N: usize> SimdState<N> {
    /// Computes the SuperTrend value and median price for one bar across `N` SIMD lanes.
    ///
    /// Updates `trend`, `prev_lb`, `prev_ub`, and `prev_st` in place.
    /// Each lane independently tracks its own trend direction and band levels.
    #[inline(always)]
    fn calc_st(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        step: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let medprice = medprice_calc_simd(high, low);
        let mut ub = medprice + step;
        let mut lb = medprice - step;

        // Trend update — element-wise, each lane independent
        let crosses_up = close.simd_gt(self.prev_st);
        let crosses_down = close.simd_lt(self.prev_st);
        self.trend = crosses_up | (self.trend & !crosses_down);

        lb = self.trend.select(self.prev_lb.simd_max(lb), lb); // uptrend: ratcheted, else: raw
        ub = self.trend.select(ub, self.prev_ub.simd_min(ub)); // uptrend: raw, else: ratcheted

        let st = self.trend.select(lb, ub);

        (self.prev_lb, self.prev_ub, self.prev_st) = (lb, ub, st);

        (st, medprice)
    }
}
