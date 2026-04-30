use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::sma::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    open: Vec<f64>,
    close: Vec<f64>,
    period: usize,
    sum: f64,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(open: &[f64], close: &[f64], sum: f64, period: usize, multiplier: f64) -> Self {
        Self {
            open: open[open.len() - period..].to_vec(),
            close: close[close.len() - period..].to_vec(),
            sum,
            period,
            multiplier,
        }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.open.extend_from_slice(inputs[0]);
        self.close.extend_from_slice(inputs[1]);

        let mut qstick_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        self.sum = cycle_qstick(
            &self.open,
            &self.close,
            self.period,
            self.multiplier,
            &mut qstick_line,
            self.sum,
        );

        self.close.drain(..self.close.len() - self.period);
        self.open.drain(..self.open.len() - self.period);

        Ok(vec![qstick_line])
    }
}

/// Returns information about the QStick indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the QStick indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "qstick",
        full_name: "QStick",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["open", "close"],
        options: &["period"],
        outputs: &["qstick"],
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
/// Calculates the QStick indicator values.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (open and close prices).
/// * `options` - A slice containing the options for the QStick calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the QStick indicator values and the state.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let open = inputs[0];
    let close = inputs[1];

    let mut qstick_line = {
        let capacity = output_length(open.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut sum = init(open, close, period);
    sum = cycle_qstick(open, close, period, multiplier, &mut qstick_line, sum);

    Ok((
        vec![qstick_line],
        IndicatorState::new(open, close, sum, period, multiplier),
    ))
}
#[inline(always)]
pub fn init(open: &[f64], close: &[f64], period: usize) -> f64 {
    let mut sum = 0.0;
    for i in 0..period {
        sum += close[i] - open[i];
    }
    sum
}
/// Calculates the QStick indicator values from the previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (open and close prices).
/// * `options` - A slice containing the options for the QStick calculation.
/// * `indicator_state` - The previous state of the QStick indicator.
/// * `_optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the QStick indicator values and the updated state.

/// Performs the main calculation loop for the QStick indicator.
///
/// # Arguments
///
/// * `open` - A slice containing the open prices.
/// * `close` - A slice containing the close prices.
/// * `period` - The period for the QStick calculation.
/// * `qstick_line` - A mutable vector to store the QStick values.
/// * `sum` - The initial sum of the differences between close and open prices.
/// * `start` - The starting index for the calculation.
///
/// # Returns
///
/// The updated sum of the differences between close and open prices.
fn cycle_qstick(
    open: &[f64],
    close: &[f64],
    period: usize,
    multiplier: f64,
    qstick_line: &mut [f64],
    mut sum: f64,
) -> f64 {
    for (j, i) in (period..open.len()).enumerate() {
        unsafe {
            *qstick_line.get_unchecked_mut(j) = calc(
                *open.get_unchecked(i),
                *close.get_unchecked(i),
                *open.get_unchecked(j),
                *close.get_unchecked(j),
                &mut sum,
                multiplier,
            );
        }
    }

    sum
}
/// Calculates the QStick value for a single bar of data.
///
/// # Arguments
///
/// * `open` - The open price for the current bar.
/// * `close` - The close price for the current bar.
/// * `prev_open` - The open price for the previous bar.
/// * `prev_close` - The close price for the previous bar.
/// * `sum` - The current sum of the differences between close and open prices.
/// * `multiplier` - The multiplier for the QStick calculation.
///
/// # Returns
///
/// A tuple containing the QStick value and the updated sum.
#[inline(always)]
pub fn calc(
    open: f64,
    close: f64,
    prev_open: f64,
    prev_close: f64,
    sum: &mut f64,
    multiplier: f64,
) -> f64 {
    let mut s = *sum;
    s += (close - open) - (prev_close - prev_open);
    *sum = s;
    s * multiplier
}
