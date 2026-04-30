use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 4;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::avgprice_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::avgprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the AvgPrice indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AvgPrice indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "avgprice",
        full_name: "Average Price",
        indicator_type: IndicatorType::Other,
        display_type: DisplayType::Overlay,
        inputs: &["open", "high", "low", "close"],
        options: &[],
        outputs: &["avgprice"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(_options: &[f64], _decimals: usize) -> usize {
    min_data(_options)
}
/// Returns the minimum amount of data required for the AvgPrice indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AvgPrice calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the AvgPrice calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the AvgPrice indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, close, and open prices.
/// * `_options` - A slice containing the options for the AvgPrice calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the AvgPrice line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    let outputs = process(inputs)?;
    Ok((outputs, IndicatorState))
}
#[inline(always)]
fn process(inputs: &[&[f64]]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;

    let open = inputs[0];
    let high = inputs[1];
    let low = inputs[2];
    let close = inputs[3];

    let mut avgprice_line = crate::uninit_vec!(f64, open.len());

    for (i, (((&o, &h), &l), &c)) in open
        .iter()
        .zip(high.iter())
        .zip(low.iter())
        .zip(close.iter())
        .enumerate()
    {
        unsafe { *avgprice_line.get_unchecked_mut(i) = calc(o, h, l, c) };
    }

    Ok(vec![avgprice_line])
}

#[inline(always)]
pub fn calc(open: f64, high: f64, low: f64, close: f64) -> f64 {
    (open + high + low + close) * 0.25
}
