use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::tr_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::tr_simd::indicator_by_assets as indicator;
}

/// Returns information about the True Range (TR) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TR indicator.

#[derive(Debug, Serialize, Deserialize)]
pub struct IndicatorState {
    prev_close: f64,
}
impl IndicatorState {
    pub fn new(prev_close: f64) -> Self {
        Self { prev_close }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let mut tr_line = crate::uninit_vec!(f64, high.len());

        cycle_tr(high, low, close, self.prev_close, 0, &mut tr_line);
        self.prev_close = close[close.len() - 1];
        Ok(vec![tr_line])
    }
}
pub fn info() -> Info<'static> {
    Info {
        name: "TR",
        full_name: "True Range",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["tr"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the TR indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the TR calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    2
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the TR calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len - 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut tr_line = {
        let capacity = output_length(high.len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle_tr(high, low, close, close[0], 1, &mut tr_line);

    Ok((
        vec![tr_line],
        IndicatorState {
            prev_close: close[close.len() - 1],
        },
    ))
}

/// Performs the main calculation loop for the TR indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `prev_close` - The previous close price.
/// * `start` - The starting index for the calculation.
/// * `tr_line` - the indicator for the output.
///
/// # Returns
///
/// A vector containing the TR line.
#[inline(always)]
fn cycle_tr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    mut prev_close: f64,
    start: usize,
    tr_line: &mut [f64],
) {
    if high.len() != low.len() || high.len() != close.len() || high.len() - start != tr_line.len() {
        return;
    }
    for (j, i) in (start..high.len()).enumerate() {
        unsafe {
            *tr_line.get_unchecked_mut(j) =
                calc(*high.get_unchecked(i), *low.get_unchecked(i), prev_close);
            prev_close = *close.get_unchecked(i);
        }
    }
}
/// Calculates the current value of the True Range (TR).
///
/// # Arguments
///
/// * `inputs` - A tuple containing the current high and low prices.
/// * `prev_close` - The previous close price.
///
/// # Returns
///
/// The current TR value.
#[inline(always)]
pub fn calc(high: f64, low: f64, prev_close: f64) -> f64 {
    let hc = (high - prev_close).abs();
    let lc = (low - prev_close).abs();

    // Use branching like C instead of max()
    let mut tr = high - low;
    if hc > tr {
        tr = hc;
    }
    if lc > tr {
        tr = lc;
    }

    tr
}
