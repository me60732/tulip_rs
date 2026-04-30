use crate::indicators::kvo::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::kvo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::kvo::indicator_by_options;

use crate::indicators::simd_indicators::{ema_simd::calc_simd as calc_ema_simd, simd_types::F64Constants};
use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    num::SimdFloat,
    *,
};
pub struct SimdState<const N: usize> {
    pub short_ema: Simd<f64, N>,
    pub long_ema: Simd<f64, N>,
    pub cm: Simd<f64, N>,
    pub trend: Simd<f64, N>,
    pub prev_hlc: Simd<f64, N>,
    pub prev_high: Simd<f64, N>,
    pub prev_low: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut short_ema = [0.0; N];
        let mut long_ema = [0.0; N];
        let mut cm = [0.0; N];
        let mut trend = [0.0; N];
        let mut prev_hlc = [0.0; N];
        let mut prev_high = [0.0; N];
        let mut prev_low = [0.0; N];

        for i in 0..N {
            short_ema[i] = states[i].short_ema;
            long_ema[i] = states[i].long_ema;
            cm[i] = states[i].cm;
            trend[i] = states[i].trend;
            prev_hlc[i] = states[i].prev_hlc;
            prev_high[i] = states[i].prev_high;
            prev_low[i] = states[i].prev_low;
        }

        Self {
            short_ema: Simd::from_array(short_ema),
            long_ema: Simd::from_array(long_ema),
            cm: Simd::from_array(cm),
            trend: Simd::from_array(trend),
            prev_hlc: Simd::from_array(prev_hlc),
            prev_high: Simd::from_array(prev_high),
            prev_low: Simd::from_array(prev_low),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let (short_ema, long_ema, cm, trend, prev_hlc, prev_high, prev_low) = (
            self.short_ema.to_array(),
            self.long_ema.to_array(),
            self.cm.to_array(),
            self.trend.to_array(),
            self.prev_hlc.to_array(),
            self.prev_high.to_array(),
            self.prev_low.to_array(),
        );

        let states: [State; N] = std::array::from_fn(|i| {
            State::new(
                short_ema[i],
                long_ema[i],
                trend[i],
                cm[i],
                prev_hlc[i],
                prev_high[i],
                prev_low[i],
            )
        });

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let (short_ema, long_ema, cm, trend, prev_hlc, prev_high, prev_low) = (
            self.short_ema.to_array(),
            self.long_ema.to_array(),
            self.cm.to_array(),
            self.trend.to_array(),
            self.prev_hlc.to_array(),
            self.prev_high.to_array(),
            self.prev_low.to_array(),
        );

        for (i, state) in states.iter_mut().enumerate() {
            state.short_ema = short_ema[i];
            state.long_ema = long_ema[i];
            state.cm = cm[i];
            state.trend = trend[i];
            state.prev_hlc = prev_hlc[i];
            state.prev_high = prev_high[i];
            state.prev_low = prev_low[i];
        }
    }
}

#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    inputs: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    multipliers: ((Simd<f64, N>, Simd<f64, N>), (Simd<f64, N>, Simd<f64, N>)),
) -> Simd<f64, N> {
    // Extract multipliers once (minor optimization)

    let vf = calc_vf_simd(inputs, state);
    let (short_multiplier, long_multiplier) = multipliers;
    state.short_ema = calc_ema_simd(vf, state.short_ema, short_multiplier);
    state.long_ema = calc_ema_simd(vf, state.long_ema, long_multiplier);
    state.short_ema - state.long_ema
}

#[inline(always)]
fn calc_vf_simd<const N: usize>(
    inputs: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    state: &mut SimdState<N>,
) -> Simd<f64, N> {
    let (high, low, close, volume) = inputs;

    let hlc = high + low + close;
    let dm = high - low;

    let hlc_up_condition = hlc.simd_gt(state.prev_hlc) & state.trend.simd_ne(F64Constants::ONE);
    let hlc_down_condition =
        hlc.simd_lt(state.prev_hlc) & state.trend.simd_ne(F64Constants::NEG_ONE);
    let should_update_cm = hlc_up_condition | hlc_down_condition;

    state.trend = hlc_down_condition.select(
        F64Constants::NEG_ONE,
        hlc_up_condition.select(F64Constants::ONE, state.trend),
    );

    // ONLY calculate new_cm when actually needed (using the mask as a guard)
    let new_cm = should_update_cm.select(
        state.prev_high - state.prev_low, // Calculate only when mask is true
        state.cm,                         // Dummy value when not needed
    );
    state.cm = should_update_cm.select(new_cm, state.cm);

    state.cm += dm.simd_max(F64Constants::EPSILON);
    state.prev_hlc = hlc;
    state.prev_high = high;
    state.prev_low = low;

    volume
        * (dm / state.cm)
            .mul_add(F64Constants::TWO, F64Constants::NEG_ONE)
            .abs()
        * F64Constants::HUNDRED
        * state.trend
}
