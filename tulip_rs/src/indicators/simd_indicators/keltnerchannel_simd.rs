#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::keltnerchannel::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::keltnerchannel::indicator_by_options;

use crate::indicators::keltnerchannel::State;
pub use crate::indicators::simd_indicators::{
    atr_simd::SimdState as AtrSimdState, ema_simd::calc_simd as ema_calc_simd,
};
use std::simd::Simd;
/// SIMD-parallel state for computing the Keltner Channel across `N` assets or option-set lanes.
///
/// Holds a Wilder ATR state and an EMA value for each lane packed into SIMD vectors.
pub struct SimdState<const N: usize> {
    pub atr_state: AtrSimdState<N>,
    pub ema: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each field into a SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        // Create vectors to collect the references
        let mut atr_refs = Vec::with_capacity(N);
        let mut ema = [0.0; N];

        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            atr_refs.push(&mut state.atr_state);
            ema[i] = state.ema;
        }

        let atr_state = AtrSimdState::new(&mut atr_refs);

        Self {
            atr_state,
            ema: Simd::from_array(ema),
        }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut atr_refs = Vec::with_capacity(N);
        let ema = self.ema.to_array();

        // Collect references and values
        for (i, state) in states.iter_mut().enumerate() {
            atr_refs.push(&mut state.atr_state);
            state.ema = ema[i];
        }
        self.atr_state.write_states(&mut atr_refs);
    }
    /// Advances the Keltner Channel by one bar across `N` lanes simultaneously.
    ///
    /// Updates the ATR and EMA states for all lanes in parallel, then computes
    /// the lower and upper channel bands as `EMA ± step × ATR`.
    ///
    /// # Arguments
    ///
    /// * `high` / `low` / `close` - SIMD vectors of current bar prices, one value per lane.
    /// * `step` - Per-lane ATR multiplier controlling channel width.
    /// * `multipliers` - `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))` per lane.
    ///
    /// # Returns
    ///
    /// `(lower, middle, upper, atr, tr)` as SIMD vectors, one value per lane,
    /// where `middle` is the current EMA of close.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
        step: Simd<f64, N>,
        multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
    ) -> (
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
        Simd<f64, N>,
    ) {
        let (atr, tr) = self.atr_state.calc_simd(high, low, close, multipliers.0);
        self.ema = ema_calc_simd(close, self.ema, multipliers.1);

        let per = atr * step;
        let upper = self.ema + per;
        let lower = self.ema - per;
        //let upper = atr.mul_add(step, self.ema);
        //let lower = atr.mul_add(-step, self.ema);

        (lower, self.ema, upper, atr, tr)
    }
}
