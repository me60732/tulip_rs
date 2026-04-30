use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::sma_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::sma_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::sma_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::sma_simd::indicator_by_options as indicator;
}


#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    //state: State,
    multiplier: f64,
    sum: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, multiplier: f64, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            //state: State::new(sum, multiplier),
            sum,
            period,
            multiplier,
        }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    /// Calculates the Simple Moving Average (SMA) indicator, picking up where the previous calculation left off.
    ///
    /// This function is useful for scenarios where indicator data is stored in a database and you need to continue calculations from the last stored state.
    ///
    /// # Arguments
    ///
    /// * `inputs` - A slice of vectors containing the input data.
    /// * `options` - A slice containing the period for the SMA calculation.
    /// * `indicator_state` - An `IndicatorState` struct containing necessary input values.
    /// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
    ///
    /// # Returns
    ///
    /// A vector of vectors containing the SMA line.
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let mut sma_line: Vec<f64> = crate::uninit_vec!(f64, inputs[0].len());
        self.real.extend_from_slice(inputs[0]);
        cycle_sma(
            &self.real,
            self.period,
            &mut sma_line,
            &mut self.sum,
            &self.multiplier,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![sma_line])
    }
}
/// Returns information about the Simple Moving Average (SMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the SMA indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "sma",
        full_name: "Simple Moving Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["sma"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the SMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the SMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the SMA calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub fn init_state(real: &[f64], period: usize) -> f64 {
    let mut sum = 0.0;
    for i in 0..period {
        sum += real[i];
    }
    sum
}

/// Calculates the Simple Moving Average (SMA) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the period for the SMA calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A Result<Output, IndicatorError>.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];
    let mut sum = init_state(real, period);
    let multiplier = multiplier(period);
    let mut sma_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle_sma(real, period, &mut sma_line, &mut sum, &multiplier);

    Ok((
        vec![sma_line],
        IndicatorState::new(real, sum, multiplier, period),
    ))
}

/// Performs the main calculation loop for the SMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the SMA calculation.
/// * `sum` - The sum of the previous input values.
/// * `start` - The starting index for the calculation.
/// * `sma_line` - A mutable reference to a vector for storing the SMA line.
fn cycle_sma(real: &[f64], period: usize, sma_line: &mut [f64], sum: &mut f64, multiplier: &f64) {
    //let multiplier = &multiplier(period);
    for (j, i) in (period..real.len()).enumerate() {
        let sma = unsafe {
            calc(
                sum,
                real.get_unchecked(i),
                real.get_unchecked(j),
                multiplier,
            )
        };
        unsafe { *sma_line.get_unchecked_mut(j) = sma };
    }
}
/// Calculates the current value of the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `sum` - The sum of the previous input values.
/// * `value` - The current input value.
/// * `prev_value` - The previous input value.
/// * `period` - The period for the SMA calculation.
///
/// # Returns
///
/// A tuple containing the current SMA value and the updated sum of the input values.
#[inline(always)]
pub fn calc(sum: &mut f64, value: &f64, prev_value: &f64, multiplier: &f64) -> f64 {
    let mut s = *sum;
    s = s + (value - prev_value);
    *sum = s;
    s * multiplier
    
}
/// Calculates the multiplier for the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `period` - The period for the SMA calculation.
///
/// # Returns
///
/// The multiplier for the SMA calculation.
#[inline(always)]
pub fn multiplier(period: usize) -> f64 {
    1.0 / period as f64
}


