use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};
//use wide::*;

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;
pub const OUTPUTS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::ema_simd::indicator_by_options as indicator;
}

pub type IndicatorState = State<Warm>;

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real: &[f64] = inputs[0];
        let mut ema_line = crate::uninit_vec!(f64, real.len());
        cycle(real, self, &mut ema_line);
        Ok(vec![ema_line])
    }
}




#[derive(Default, Serialize, Deserialize, Copy, Clone)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub ema: f64,
    pub inv_multiplier: f64,
    pub multiplier: f64,
    pub(crate) state: std::marker::PhantomData::<S>
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        self.ema = calc(value, self.ema, self.multiplier, self.inv_multiplier);
        self.ema
    }
}
impl State<Cold> {
    pub fn new(ema: f64, period: usize) -> Self {
        let multipliers = multiplier(period);
        Self {
            ema,
            inv_multiplier: multipliers.1,
            multiplier: multipliers.0,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            ema: self.ema,
            inv_multiplier: self.inv_multiplier,
            multiplier: self.multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut ema, (multiplier, inv_multiplier)) = (real[0], multiplier(period));
        for i in 1..period {
            ema = calc(real[i], ema, multiplier, inv_multiplier);
        }
        State {
            ema,
            inv_multiplier,
            multiplier,
            state: std::marker::PhantomData,
        }
    }
    
}
pub struct Ema;
impl Indicator<INPUTS, OPTIONS> for Ema {
    const INFO: Info = Info {
        name: "ema",
        full_name: "Exponential Moving Average",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["ema"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "ema",
            label: "EMA",
            display_type: DisplayType::Overlay,
            outputs: &["ema"],
        }],
    };
    type IndicatorState = IndicatorState;

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
    
        let mut state = State::init_state(inputs[0], period);
        let real = &inputs[0][period..];
        let mut ema_line = {
            let capacity = Self::output_length(inputs[0].len(), &[period as f64]);
            crate::uninit_vec!(f64, capacity)
        };
    
        cycle(real, &mut state, &mut ema_line);
        Ok((vec![ema_line], state))
    }
}
fn cycle(real: &[f64], state: &mut State<Warm>, ema_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *ema_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
        }
    }
}
#[inline(always)]
pub fn calc(value: f64, prev_ema: f64, multiplier: f64, inv_multiplier: f64) -> f64 {
    //prev_ema * inv_multiplier + value * multiplier
    prev_ema.mul_add(inv_multiplier, value * multiplier)
}

#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64) {
    let per = 2.0 / (period as f64 + 1.0);
    (per, 1.0 - per)
}
