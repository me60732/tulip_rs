use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::medprice_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::medprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the Median Price (MEDPRICE) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MEDPRICE indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "medprice",
        full_name: "Median Price",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low"],
        options: &[],
        outputs: &["medprice"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the MEDPRICE indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the MEDPRICE calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1 // Only one data point is needed to calculate the median price
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the MEDPRICE calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Median Price (MEDPRICE) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high and low prices.
/// * `_options` - A slice containing the options for the MEDPRICE calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the MEDPRICE line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    let outputs = process(inputs)?;
    Ok((outputs, IndicatorState))
}
#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS_WIDTH]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;
    let high = inputs[0];
    let low = inputs[1];

    let mut medprice_line = crate::uninit_vec!(f64, high.len());

    for (i, (&high_value, &low_value)) in high.iter().zip(low.iter()).enumerate() {
        unsafe { *medprice_line.get_unchecked_mut(i) = calc(high_value, low_value) };
    }

    Ok(vec![medprice_line])
}

/// Calculates the median price.
#[inline(always)]
pub fn calc(high: f64, low: f64) -> f64 {
    0.5 * (high + low)
}
