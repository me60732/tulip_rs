//use std::vec;
use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, TState, Indicator, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::obv_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::obv_simd::indicator_by_assets as indicator;
}
pub type IndicatorState = State;
#[derive(Serialize, Deserialize)]
pub struct State {
    pub obv: f64,
    pub prev_close: f64,
}
impl State {
    pub fn new(obv: f64, prev_close: f64) -> Self {
        Self { obv, prev_close }
    }
    
}
impl TState for State {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (close, volume): Self::Inputs<'a>) -> Self::Outputs {
        if close > self.prev_close {
            self.obv += volume;
        } else if close < self.prev_close {
            self.obv -= volume
        }
        self.prev_close = close;
        self.obv
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut obv_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_obv(inputs[0], inputs[1], &mut obv_line, self);

        Ok(vec![obv_line])
    }
}


/// Iterates over the input data and applies the calc function.
//#[inline(always)]
fn cycle_obv(close: &[f64], volume: &[f64], obv_line: &mut [f64], state: &mut IndicatorState) {
    for i in 0..close.len() {
        unsafe {
            *obv_line.get_unchecked_mut(i) =
                state.calc((*close.get_unchecked(i), *volume.get_unchecked(i)));
        }
    }
}

pub struct Obv;

impl Indicator<INPUTS, OPTIONS> for Obv {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "obv",
        full_name: "On-Balance Volume",
        indicator_type: IndicatorType::Volume,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["obv"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "obv",
            label: "OBV",
            display_type: DisplayType::Indicator,
            outputs: &["obv"],
        }],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_inputs(inputs, Self::min_data(_options))?;
    
        let mut obv_line = {
            let capacity = Self::output_length(inputs[0].len(), _options);
            crate::uninit_vec!(f64, capacity)
        };
    
        let mut state = IndicatorState::new(0.0, inputs[0][0]);
        cycle_obv(&inputs[0][1..], &inputs[1][1..], &mut obv_line, &mut state);
    
        Ok((vec![obv_line], state))
    }
}