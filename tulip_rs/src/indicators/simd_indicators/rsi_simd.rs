#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::rsi::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::rsi::indicator_by_options;

use crate::indicators::rsi::State;
use crate::indicators::simd_indicators::{
    cmo_simd::up_down_simd, simd_types::F64Constants, 
    wilders_simd::{calc_simd as calc_wilders, SimdState as WildersSimdState},
};
use crate::types::Warm;
use std::simd::Simd;
pub use crate::indicator_types::{TSimdState, TState};
use std::ops::{Deref, DerefMut};
/// SIMD-parallel state for the Relative Strength Index (RSI) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub up_sum: WildersSimdState<N>,
    pub down_sum: Simd<f64, N>,
    pub prev_real: Simd<f64, N>,
}
impl<const N: usize> Deref for SimdState<N> {
    type Target = WildersSimdState<N>;
    fn deref(&self) -> &Self::Target { &self.up_sum }
}
impl<const N: usize> DerefMut for SimdState<N> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.up_sum }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    /// Constructs a `SimdState` by gathering scalar per-asset states into SIMD vectors.
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
        let mut up_sum = [0.0; N];
        let mut down_sum = [0.0; N];
        let mut prev_real = [0.0; N];
        let mut multiplier = [0.0; N];
        let mut inv_multiplier = [0.0; N];
        for i in 0..N {
            let [up, down] = states[i].wilders_state.wilders.to_array();
            up_sum[i] = up;
            down_sum[i] = down;
            prev_real[i] = states[i].prev_real;
            multiplier[i] = states[i].wilders_state.multiplier[0];
            inv_multiplier[i] = states[i].wilders_state.inv_multiplier[0];
        }
        Self {
            up_sum: WildersSimdState::new(Simd::from_array(up_sum), (Simd::from_array(multiplier), Simd::from_array(inv_multiplier))),
            down_sum: Simd::from_array(down_sum),
            prev_real: Simd::from_array(prev_real),
        }
    }

    /// Writes the current SIMD lane values back into the provided scalar per-asset states.
    fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
        let up_sum = self.up_sum.wilders.to_array();
        let down_sum = self.down_sum.to_array();
        let prev_real = self.prev_real.to_array();

        for i in 0..N {
            let [up, down] = states[i].wilders_state.wilders.as_mut_array();
            *up = up_sum[i];
            *down = down_sum[i];
            states[i].prev_real = prev_real[i];
        }
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        cur_real: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let (up, down) = up_down_simd(cur_real, self.prev_real);
        let up_sum = self.up_sum.calc(up);
        self.down_sum = calc_wilders(self.down_sum, down, (self.multiplier, self.inv_multiplier));

        self.prev_real = cur_real;

        F64Constants::HUNDRED * (up_sum / (up_sum + self.down_sum))
    }
}
