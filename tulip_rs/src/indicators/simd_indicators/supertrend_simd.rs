#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::supertrend::indicator_by_assets;
use crate::indicators::simd_indicators::{
    atr_simd::SimdState as AtrSimdState, medprice_simd::calc_simd as medprice_calc_simd,
};
use crate::indicators::supertrend::State;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::supertrend::indicator_by_options;

use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Mask, Select, Simd};

/// SIMD-parallel state for computing the SuperTrend indicator across `N` assets
/// simultaneously. Each field is a SIMD vector where lane `i` holds the value for asset `i`.
pub struct SimdState<const N: usize> {
    pub atr_state: AtrSimdState<N>,
    pub prev_st: Simd<f64, N>,
    pub prev_ub: Simd<f64, N>,
    pub prev_lb: Simd<f64, N>,
    pub trend: Mask<i64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a `SimdState`, packing each sub-state
    /// into the corresponding SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        // Create vectors to collect the references
        let mut atr_refs = Vec::with_capacity(N);
        let mut prev_st = [0.0; N];
        let mut prev_ub = [0.0; N];
        let mut prev_lb = [0.0; N];
        let mut trend = [false; N];

        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            prev_st[i] = state.prev_st;
            prev_ub[i] = state.prev_ub;
            prev_lb[i] = state.prev_lb;
            trend[i] = state.trend;
            atr_refs.push(&mut state.atr_state);
        }

        let atr_state = AtrSimdState::new(&atr_refs);

        Self {
            atr_state,
            prev_st: Simd::from_array(prev_st),
            prev_ub: Simd::from_array(prev_ub),
            prev_lb: Simd::from_array(prev_lb),
            trend: Mask::from_array(trend),
        }
    }

    /// Writes the SIMD state back into `N` existing mutable [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut atr_refs = Vec::with_capacity(N);
        let prev_st = self.prev_st.as_array();
        let prev_ub = self.prev_ub.as_array();
        let prev_lb = self.prev_lb.as_array();
        let trend = self.trend.to_array();
        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            atr_refs.push(&mut state.atr_state);
            state.prev_st = prev_st[i];
            state.prev_ub = prev_ub[i];
            state.prev_lb = prev_lb[i];
            state.trend = trend[i];
        }
        self.atr_state.write_states(&mut atr_refs);
    }
    /// Advances all `N` lanes by one bar, returning `(supertrend, atr, tr, medprice)`.
    ///
    /// Delegates ATR computation to `atr_state.calc_simd`, scales the result by `step` to
    /// obtain the band half-width, then updates per-lane trend flags and band ratchets.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        step: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (atr, tr) = self.atr_state.calc_simd(high, low, close, multipliers);
        let step = step * atr;
        let (st, medprice) = self.calc_st(high, low, close, step);
        (st, atr, tr, medprice)
    }

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
