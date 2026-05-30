use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// Returns information about the Pivot Point indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Pivot Point indicator.
pub const INFO: Info = Info {
    name: "pivotpoint",
    full_name: "Pivot Point",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["s3", "s2", "s1", "pp", "r1", "r2", "r3"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "pivotpoint",
        label: "PIVOTPOINT",
        display_type: DisplayType::Overlay,
        outputs: &["s3", "s2", "s1", "pp", "r1", "r2", "r3"],
    }],
};
#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);
        self.close.extend_from_slice(inputs[2]);
        let outputs = process(&self.high, &self.low, &self.close, self.period);

        self.high.drain(..self.high.len() - self.period + 1);
        self.low.drain(..self.low.len() - self.period + 1);
        self.close.drain(..self.close.len() - self.period + 1);

        Ok(outputs)
    }
}
/// Returns the minimum amount of data required for the Pivot Point indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Pivot Point calculation (e.g. `period`).
///
/// # Returns
///
/// The minimum number of input data points required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `_decimal_places` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimal_places: usize) -> usize {
    min_data(options)
}
/// Returns the output length for the Pivot Point indicator.
///
/// # Arguments
///
/// * `_data_len` - The length of the input data (unused; Pivot Point always returns one value).
/// * `_options` - A slice containing the options for the Pivot Point calculation.
///
/// # Returns
///
/// Always returns `1`, as only the most recent pivot point values are output.
pub fn output_length(_data_len: usize, _options: &[f64]) -> usize {
    1
}

/// Calculates the Pivot Point indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (look-back window length)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is a single-element vector
/// `[s3, s2, s1, pp, r1, r2, r3]` representing the three support levels,
/// pivot point, and three resistance levels computed from the most recent
/// `period` bars. `state` can be passed to `IndicatorState::batch_indicator`
/// for streaming. Returns `Err(IndicatorError)` if inputs are too short or
/// options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;
    let period = options[0] as usize;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];
    let outputs = process(high, low, close, period);

    Ok((
        outputs,
        IndicatorState {
            period,
            high: high[high.len() - period + 1..].to_vec(),
            low: low[low.len() - period + 1..].to_vec(),
            close: close[close.len() - period + 1..].to_vec(),
        },
    ))
}
fn process(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<Vec<f64>> {
    let start_index = high.len() - period;
    let high = &high[start_index..];
    let low = &low[start_index..];
    let close = &close[start_index..];
    let (s3, s2, s1, pp, r1, r2, r3) = calc(high, low, close);
    vec![vec![s3, s2, s1, pp, r1, r2, r3]]
}

/// Calculates the support and resistance levels for the Pivot Point indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices over the look-back period.
/// * `low` - A slice of low prices over the look-back period.
/// * `close` - A slice of close prices; the last element is used as the closing price.
///
/// # Returns
///
/// A tuple `(s3, s2, s1, pivot_point, r1, r2, r3)` of the three support levels,
/// the pivot point, and the three resistance levels.
#[inline(always)]
pub fn calc(high: &[f64], low: &[f64], close: &[f64]) -> (f64, f64, f64, f64, f64, f64, f64) {
    let close_value = close[close.len() - 1];
    let (low_value, high_value) = low
        .iter()
        .copied()
        .zip(high.iter().copied())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), (l, h)| {
            (min.min(l), max.max(h))
        });

    let pivot_point = (high_value + low_value + close_value) / 3.0;
    let s1 = (pivot_point * 2.0) - high_value;
    let s2 = pivot_point - (high_value - low_value);
    let s3 = s1 - (high_value - low_value);
    let r1 = (pivot_point * 2.0) - low_value;
    let r2 = pivot_point + (high_value - low_value);
    let r3 = r1 + (high_value - low_value);
    (s3, s2, s1, pivot_point, r1, r2, r3)
}
