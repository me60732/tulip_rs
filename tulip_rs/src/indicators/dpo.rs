use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::sma::init_state;
pub use crate::indicators::sma::multiplier;
use crate::indicators::sma::{calc as calc_sma, output_length as sma_output_length};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    multiplier: f64,
    sum: f64,
    dpo_period: usize,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, multiplier: f64, period: usize, dpo_period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            sum,
            multiplier,
            period,
            dpo_period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut dpo_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_dpo(
            &self.real,
            (self.period, self.dpo_period),
            self.multiplier,
            &mut self.sum,
            &mut dpo_line,
            &mut sma_line,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![dpo_line, sma_line])
    }
}
//Review all doc comments and rewrite (update) to reflect function defination changes
/// Returns information about the Detrended Price Oscillator (DPO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DPO indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "dpo",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Other,
        full_name: "Detrended Price Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["dpo"],
        optional_outputs: &["sma"],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the DPO indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DPO calculation.
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
/// * `options` - A slice containing the options for the DPO calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Detrended Price Oscillator (DPO) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the DPO calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the DPO line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let dpo_period = period / 2 + 1;

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut dpo_line, mut sma_line) = {
        let sma_capacity = sma_output_length(real.len(), options);
        let capacity = output_length(real.len(), options);

        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                sma_line: sma_capacity
            ),
        )
    };

    let mut sum = init_state(real, period);
    let multiplier = multiplier(period);
    // Perform the main DPO calculation
    cycle_dpo(
        real,
        (period, dpo_period),
        multiplier,
        &mut sum,
        &mut dpo_line,
        &mut sma_line,
    );

    Ok((
        vec![dpo_line, sma_line],
        IndicatorState {
            sum,
            period,
            dpo_period,
            multiplier,
            real: real[real.len() - period..].to_vec(),
        },
    ))
}

/// Performs the main calculation loop for the DPO indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the DPO calculation.
/// * `sum` - The sum of the previous input values.
/// * `start` - The starting index for the calculation.
/// * `dpo_line` - A mutable reference to a vector for storing the DPO line.
/// * `output_vectors` - A mutable reference to a slice of optional output vectors.
fn cycle_dpo(
    real: &[f64],
    periods: (usize, usize),
    multiplier: f64,
    sum: &mut f64,
    dpo_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (period, dpo_period) = periods;
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (value, prev_values);
        unsafe {
            value = real.get_unchecked(i);
            prev_values = (
                real.get_unchecked(j),
                real.get_unchecked(i - dpo_period),
            );
        }
        let (dpo, sma) = calc(value, sum, prev_values, multiplier);
        unsafe {
            *dpo_line.get_unchecked_mut(j) = dpo;
        }
        crate::store_optional_outputs!(j, want_sma, sma_line => sma);
    }
}

/// Calculates the Detrended Price Oscillator (DPO) for the current data point.
///
/// # Arguments
///
/// * `value` - The current data point.
/// * `sum` - The sum of the previous input values.
/// * `prev_value` - The previous input value.
/// * `period` - The period for the DPO calculation.
///
/// # Returns
///
/// A tuple `(dpo,sma, sum)` representing the DPO and the updated sum.
#[inline(always)]
pub fn calc(value: &f64, sum: &mut f64, prev_values: (&f64, &f64), multiplier: f64) -> (f64, f64) {
    //let (sma, mut s) = (0.0, *sum);
    let (prev_value, dpo_price) = prev_values;
    let sma = calc_sma(sum, value, prev_value, &multiplier);
    (dpo_price - sma, sma)
}
