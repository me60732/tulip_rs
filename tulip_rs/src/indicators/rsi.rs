use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub(crate) use crate::indicators::cmo::up_down;
pub use crate::indicators::sma::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options as indicator;
}

/// Returns information about the Relative Strength Index (RSI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the RSI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "rsi",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        full_name: "Relative Strength Index",
        inputs: &["real"],
        options: &["period"],
        outputs: &["rsi"],
        optional_outputs: &[],
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(state: State, multiplier: f64) -> Self {
        Self { state, multiplier }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut rsi_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_rsi(
            inputs[0],
            self.multiplier,
            &mut rsi_line,
            &mut self.state,
        );

        Ok(vec![rsi_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub up_sum: f64,
    pub down_sum: f64,
    pub prev_real: f64,
}
impl State {
    pub fn new(prev_real: f64, up_sum: f64, down_sum: f64) -> Self {
        Self {
            prev_real,
            up_sum,
            down_sum,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let (up, down) = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        up_sum /= period as f64;
        down_sum /= period as f64;

        Self {
            up_sum,
            down_sum,
            prev_real: real[period],
        }
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize)],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the RSI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the RSI calculation.
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
/// * `options` - A slice containing the options for the RSI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options)
}

/// Calculates the Relative Strength Index (RSI) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the real prices.
/// * `options` - A slice containing the options for the RSI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// A vector of vectors containing the RSI line.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let mut rsi_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(inputs[0], period);

    cycle_rsi(&inputs[0][period+1..], multiplier, &mut rsi_line, &mut state);

    Ok((vec![rsi_line], IndicatorState { multiplier, state }))
}

/// Performs the main calculation loop for the RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `period` - The period for the RSI calculation.
/// * `rsi_line` - A mutable reference to a vector for storing the RSI line.
fn cycle_rsi(real: &[f64], multiplier: f64, rsi_line: &mut [f64], state: &mut State) {
    for i in 0..real.len() {
        unsafe { *rsi_line.get_unchecked_mut(i) = calc(state, *real.get_unchecked(i), multiplier) };
    }
}
#[inline(always)]
pub fn calc(state: &mut State, cur_real: f64, multiplier: f64) -> f64 {
    let (mut up_sum, mut down_sum) = (state.up_sum, state.down_sum);
    let (up, down) = up_down(cur_real, state.prev_real);

    //up_sum = (up - up_sum) * multiplier + up_sum;
    up_sum = (up - up_sum).mul_add(multiplier, up_sum);
    //down_sum = (down - down_sum) * multiplier + down_sum;
    down_sum = (down - down_sum).mul_add(multiplier, down_sum);

    (state.up_sum, state.down_sum, state.prev_real) = (up_sum, down_sum, cur_real);

    100.0 * (up_sum / (up_sum + down_sum))
}
