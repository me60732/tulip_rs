use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::typprice_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::typprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the Typical Price (TYPPRICE) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TYPPRICE indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "TYPPRICE",
        full_name: "Typical Price",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["typprice"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the TYPPRICE indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the TYPPRICE calculation.
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
/// * `_options` - A slice containing the options for the TYPPRICE calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Typical Price (TYPPRICE) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `_options` - A slice containing the options for the TYPPRICE calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the TYPPRICE line.

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
    let close = inputs[2];

    let mut typprice_line = crate::uninit_vec!(f64, high.len()); // Vec::with_capacity(capacity);

    cycle_typprice((high, low, close), &mut typprice_line);

    Ok(vec![typprice_line])
}
/// Calculates the Typical Price (TYPPRICE) indicator, picking up where the previous calculation left off.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `_options` - A slice containing the options for the TYPPRICE calculation.
/// * `indicator_state` - An `IndicatorState` struct containing necessary input values.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the TYPPRICE line.

/// Performs the main calculation loop for the TYPPRICE indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `typprice_line` - A mutable reference to a vector for storing the TYPPRICE line.
#[inline(always)]
fn cycle_typprice(inputs: (&[f64], &[f64], &[f64]), typprice_line: &mut [f64]) {
    let (high, low, close) = inputs;
    for i in 0..high.len() {
        unsafe {
            *typprice_line.get_unchecked_mut(i) = calc(
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            );
        }
    }
}

/// Calculates the Typical Price (TYPPRICE) value.
///
/// # Arguments
///
/// * `high` - The high price.
/// * `low` - The low price.
/// * `close` - The close price.
///
/// # Returns
///
/// The TYPPRICE value.
const DIV: f64 = 1.0 / 3.0;
#[inline(always)]
pub fn calc(high: &f64, low: &f64, close: &f64) -> f64 {
    (high + low + close) * DIV
}
