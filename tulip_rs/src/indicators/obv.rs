//use std::vec;
use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::obv_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::obv_simd::indicator_by_assets as indicator;
}

/// Returns information about the On-Balance Volume (OBV) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "obv",
        full_name: "On-Balance Volume",
        indicator_type: IndicatorType::Volume,
        display_type: DisplayType::Indicator,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["obv"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub obv: f64,
    pub prev_close: f64,
}
impl IndicatorState {
    pub fn new(obv: f64, prev_close: f64) -> Self {
        Self { obv, prev_close }
    }
    /// Performs the core calculation for the On-Balance Volume (OBV) indicator.
    #[inline(always)]
    pub fn calc(&mut self, close: f64, volume: f64) -> f64 {
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
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut obv_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_obv(inputs[0], inputs[1], &mut obv_line, self);

        Ok(vec![obv_line])
    }
}
pub fn min_data_accuracy(_options: &[f64], _decimals: usize) -> usize {
    min_data(_options)
}
/// Returns the minimum amount of data required for the OBV indicator.
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Returns the output length for the OBV indicator.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len - min_data(_options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let mut obv_line = {
        let capacity = output_length(inputs[0].len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = IndicatorState::new(0.0, inputs[0][0]);
    cycle_obv(&inputs[0][1..], &inputs[1][1..], &mut obv_line, &mut state);

    Ok((vec![obv_line], state))
}

/// Iterates over the input data and applies the calc function.
//#[inline(always)]
fn cycle_obv(
    close: &[f64],
    volume: &[f64],
    obv_line: &mut [f64],
    state: &mut IndicatorState,
) {
    for i in 0..close.len() {
        unsafe {
            *obv_line.get_unchecked_mut(i) = state.calc(*close.get_unchecked(i), *volume.get_unchecked(i));
        }
    }
}

