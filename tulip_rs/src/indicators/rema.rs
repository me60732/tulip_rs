use crate::common::validate_inputs;
use crate::types::{IndicatorState, Info, IndicatorError, IndicatorType, DisplayType, Output};
use crate::indicators::range::calc as range_calc;
use crate::indicators::ema::{calc as ema_calc, output_length as ema_output_length, min_data as ema_min_data, min_data_accuracy as ema_min_data_accuracy, multiplier as ema_multiplier};
pub const INPUTS_WIDTH: usize = 2;
/// Returns information about the Accumulation/Distribution Line (AD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AD indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "rema",
        full_name: "Range EMA",
        indicator_type: IndicatorType::Volatility,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["ema"],
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
pub fn min_data(options: &[f64]) -> usize {
    ema_min_data(options)
}
pub fn min_data_accuracy(options: &[f64], decimal_places: usize) -> usize {
    ema_min_data_accuracy(options, decimal_places)
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

pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    ema_output_length(data_len, options)
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
pub fn indicator(inputs: &[&[f64]; INPUTS_WIDTH], options: &[f64; 1], _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {

    if options[0] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    let period = options[0] as usize;
    
    validate_inputs(inputs, min_data(options))?;
    let high = inputs[0];
    let low = inputs[1];
    let mut ema = init_state(high, low, period);
    let capacity = output_length(high.len(), options);
    let mut range_line = vec![0.0; capacity];//Vec::with_capacity(capacity);

    ema = cycle(high, low, &mut range_line, period, ema, period);

    Ok(Output {
        indicators: vec![range_line],
        state: IndicatorState{
            array_values: None,
            single_values: Some(vec![ema])
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
pub fn indicator_from_state(inputs: &[&[f64]; INPUTS_WIDTH], options: &[f64; 1], indicator_state: &IndicatorState, _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {
    if options[0] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    let period = options[0] as usize;
    validate_inputs(inputs, min_data(options))?;

    let high = inputs[0];
    let low = inputs[1];
    let ema = indicator_state.single_values()[0];
    let mut range_line = vec![0.0; high.len()];//Vec::with_capacity(high.len());

    cycle(high, low,  &mut range_line, 0, ema, period);

    Ok(Output {
        indicators: vec![range_line],
        state: IndicatorState{
            array_values: None,
            single_values: None
        }
    })
}

/// Performs the main calculation loop for the AD indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `start` - The starting index for the calculation.
pub(crate)fn cycle(high: &[f64], low: &[f64], range_line: &mut [f64], start: usize, mut ema: f64, period: usize) -> f64 {

    let multiplier = multiplier(period);
    //high.iter().skip(start).zip(low.iter()).for_each(|(&high_val, &low_val)| {
    for (j, i) in (start..high.len()).enumerate() {
        ema = calc(high[i], low[i], ema, multiplier);
        range_line[j] = ema;
    }//);
    ema
}
pub fn init_state(high: &[f64], low: &[f64], period: usize) -> f64 {
    let mut ema = high[0] - low[0];
    let multiplier = multiplier(period);
    for i in 1..period {
        ema = calc(high[i], low[i], ema, multiplier);
    }
    ema
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
pub fn calc(high: f64, low: f64, ema: f64, multiplier: (f64, f64)) -> f64 {
    let range = &range_calc(&high, &low);
    ema_calc(range, ema, multiplier)
}
#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64) {
    ema_multiplier(period)
}
