use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 1;
/// Returns information about the Accumulation/Distribution Line (AD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AD indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "pivitpoint",
        full_name: "Pivit Point",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Overlay,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["s3", "s2", "s1", "pp", "r1", "r2", "r3"],
        optional_outputs: &[],
    }
}
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
/// Returns the minimum amount of data required for the AD indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AD calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}
pub fn min_data_accuracy(options: &[f64], _decimal_places: usize) -> usize {
    min_data(options)
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the AD calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.

pub fn output_length(_data_len: usize, _options: &[f64]) -> usize {
    1
}

/// Calculates the Accumulation/Distribution Line (AD) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, close prices, and volume.
/// * `_options` - A slice containing the options for the AD calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data, keep in mind with most indicators this is speed vs accuracy.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A `Result` containing a vector of vectors with the AD line or an `IndicatorError`.
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

/// Calculates the current value of the s3, s2, s1, pivot_point, r1, r2, r3 piviot point indicator.
///
/// # Arguments
///
/// * `prev_ad` - The previous AD value.
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `volume` - The current volume.
///
/// # Returns
///
/// The updated AD value.
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
