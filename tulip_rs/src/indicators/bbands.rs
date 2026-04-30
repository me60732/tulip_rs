use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::stddev::calc as stddev_calc;
pub use crate::indicators::stddev::{multiplier, State};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 2;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options as indicator;
}

/// Returns information about the Bollinger Bands (BBANDS) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the BBANDS indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "bbands",
        full_name: "Bollinger Bands",
        indicator_type: IndicatorType::Volatility,
        display_type: DisplayType::Overlay,
        inputs: &["real"],
        options: &["period", "std_dev"],
        outputs: &["bbands_lower", "bbands_middle", "bbands_upper"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State,
    period: usize,
    multiplier: f64,
    std_dev: f64,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, period: usize, multiplier: f64, std_dev: f64) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
            multiplier,
            std_dev,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        let period = self.period;

        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut middle_band, mut upper_band, mut lower_band) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        cycle_bbands(
            &self.real,
            period,
            self.std_dev,
            self.multiplier,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut self.state,
        );

        self.real.drain(..self.real.len() - period);

        Ok(vec![lower_band, middle_band, upper_band])
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the BBANDS indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the BBANDS calculation.
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
/// * `_options` - A slice containing the options for the BBANDS calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Bollinger Bands (BBANDS) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the real prices.
/// * `_options` - A slice containing the options for the BBANDS calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the lower band, middle band, and upper band.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let std_dev = options[1];

    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut middle_band, mut upper_band, mut lower_band) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        )
    };

    let mut state = State::new(
        real[0..period].iter().sum::<f64>(),
        real[0..period].iter().map(|&x| x * x).sum::<f64>(),
    );
    cycle_bbands(
        real,
        period,
        std_dev,
        multiplier,
        (&mut lower_band, &mut middle_band, &mut upper_band),
        &mut state,
    );

    Ok((
        vec![lower_band, middle_band, upper_band],
        IndicatorState {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
            multiplier,
            std_dev,
        },
    ))
}

/// Performs the main calculation loop for the BBANDS indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `period` - The period for the BBANDS calculation.
/// * `std_dev` - The standard deviation multiplier for the BBANDS calculation.
/// * `middle_band` - A mutable reference to a vector for storing the middle band.
/// * `upper_band` - A mutable reference to a vector for storing the upper band.
/// * `lower_band` - A mutable reference to a vector for storing the lower band.
/// * `sum` - The sum of the previous input values.
/// * `sum_sq` - The sum of the squares of the previous input values.
/// * `start` - The starting index for the calculation.
#[inline(always)]
fn cycle_bbands(
    real: &[f64],
    period: usize,
    std_dev: f64,
    multiplier: f64,
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State,
) {
    let (lower_band, middle_band, upper_band) = outputs;

    for (j, i) in (period..real.len()).enumerate() {
        let prev_value = unsafe { real.get_unchecked(i - period) };
        //let prev_value = &real[i - period];
        let (lower, middle, upper) = calc(state, &std_dev, multiplier, &real[i], prev_value);
        unsafe {
            *middle_band.get_unchecked_mut(j) = middle;
            *upper_band.get_unchecked_mut(j) = upper;
            *lower_band.get_unchecked_mut(j) = lower;
        }
    }
}

/// Calculates the current Bollinger Bands values.
///
/// # Arguments
///
/// * `std_dev` - The standard deviation multiplier for the BBANDS calculation.
/// * `sum_sq` - The sum of the squares of the previous input values.
/// * `sum` - The sum of the previous input values.
/// * `period` - The period for the BBANDS calculation.
/// * `value` - The current input value.
/// * `prev_value` - The previous input value.
///
/// # Returns
///
/// A tuple containing the lower band, middle band, upper band, sum, sum_sq.
#[inline(always)]
pub fn calc(
    state: &mut State,
    std_dev: &f64,
    multiplier: f64,
    value: &f64,
    prev_value: &f64,
) -> (f64, f64, f64) {
    let (sd, sma);
    (sd, sma) = stddev_calc(state, value, prev_value, multiplier);

    let upper_band = std_dev.mul_add(sd, sma);
    //let upper_band = sma + std_dev * sd;
    //let lower_band = sma - std_dev * sd;
    let lower_band = (-std_dev).mul_add(sd, sma);
    (lower_band, sma, upper_band)
}
