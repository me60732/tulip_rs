use crate::common::validate_inputs;
use crate::types::{IndicatorState, Info, IndicatorError, IndicatorType, DisplayType, Output};
pub const INPUTS_WIDTH: usize = 2;
/// Returns information about the Accumulation/Distribution Line (AD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AD indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "range",
        full_name: "Range",
        indicator_type: IndicatorType::Volatility,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low"],
        options: &[],
        outputs: &["range"],
        optional_outputs: &[]
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
pub fn min_data(_options: &[f64]) -> usize {
    1
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

pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
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
pub fn indicator(inputs: &[&[f64]; INPUTS_WIDTH], _options: &[f64; 0], _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {

    validate_inputs(inputs, min_data(_options))?;
    let high = inputs[0];
    let low = inputs[1];
    
    let mut range_line = vec![0.0; high.len()];//Vec::with_capacity(high.len());

    cycle(high, low, &mut range_line);

    Ok(Output {
        indicators: vec![range_line],
        state: IndicatorState{
            array_values: None,
            single_values: None
        }
    })
    
}

/// Calculates the Range indicator, picking up where the previous calculation left off.
///
/// This function is useful for scenarios where indicator data is stored in a database and you need to continue calculations from the last stored state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, close prices, and volume.
/// * `_options` - A slice containing the options for the AD calculation.
/// * `indicator_state` - An `IndicatorState` struct containing necessary input values.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
// A `Result` containing a vector of vectors with the AD line or an `IndicatorError`.
pub fn indicator_from_state(inputs: &[&[f64]; 2], _options: &[f64; 0], _indicator_state: &IndicatorState, _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {
    let result = indicator(inputs, _options, _optional_outputs)?;
    Ok(result)
}

/// Performs the main calculation loop for the AD indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `start` - The starting index for the calculation.
fn cycle(high: &[f64], low: &[f64], range_line: &mut [f64]) {
    for i in 0..high.len() {
        let range = calc(&high[i], &low[i]);
        range_line[i] = range;
    }
}

/// Calculates the current value of the Range indicator.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
///
/// # Returns
///
/// The updated Range value.
#[inline(always)]
pub fn calc(high: &f64, low: &f64) -> f64 {
    (high - low).abs()
}