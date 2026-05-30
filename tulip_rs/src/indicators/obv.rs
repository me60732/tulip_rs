//use std::vec;
use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

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

/// Returns information about the On-Balance Volume (OBV) indicator.
pub const INFO: Info = Info {
    name: "obv",
    full_name: "On-Balance Volume",
    indicator_type: IndicatorType::Volume,
    inputs: &["close", "volume"],
    options: &[],
    outputs: &["obv"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "obv",
        label: "OBV",
        display_type: DisplayType::Indicator,
        outputs: &["obv"],
    }],
};
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
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `_options` - Unused; this indicator takes no options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
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

/// Calculates the On-Balance Volume (OBV) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close prices
/// * `inputs[1]` — volume
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `_options` - Unused; this indicator takes no options.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `obv` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short.
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
fn cycle_obv(close: &[f64], volume: &[f64], obv_line: &mut [f64], state: &mut IndicatorState) {
    for i in 0..close.len() {
        unsafe {
            *obv_line.get_unchecked_mut(i) =
                state.calc(*close.get_unchecked(i), *volume.get_unchecked(i));
        }
    }
}
