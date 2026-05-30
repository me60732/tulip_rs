use crate::candle_indicators::candle_patterns::*;
use crate::candle_indicators::pattern_test::{State, MAX_PATTERN_LENGTH};
pub use crate::candle_indicators::types::ForecastType;
use crate::common::{validate_inputs, validate_options};
use crate::indicators::ema::{min_data as ema_min_data, multiplier as ema_multiplier};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 3;

/// Returns information about the Candlestick Pattern indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Candlestick Pattern indicator.
pub const INFO: Info = Info {
    name: "candlestick",
    full_name: "Candle Stick Indicator",
    indicator_type: IndicatorType::CandleStick,
    inputs: &["open", "high", "low", "close"],
    options: &["candle_period", "trend_period", "trend_signal_period"],
    outputs: &["cdl_pattern"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "candlestick",
        label: "CANDLESTICK",
        display_type: DisplayType::Overlay,
        outputs: &["cdl_pattern"],
    }],
};

/// Returns the minimum amount of data required for the Candlestick Pattern indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[candle_period, trend_period, trend_signal_period]`.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    if options[0] > options[1] + options[2] {
        return ema_min_data(&[options[0]]) + MAX_PATTERN_LENGTH;
    }
    ema_min_data(&[options[1]]) + options[2] as usize + MAX_PATTERN_LENGTH + 1
}
/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Candlestick Pattern calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    //println!("Len: {:?}, Options: {:?}", data_len, options);
    data_len - min_data(options) + 1
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
}
impl IndicatorState {
    pub fn new(state: State, open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Self {
        Self {
            state,
            open: open[open.len() - MAX_PATTERN_LENGTH - 1..].to_vec(),
            high: high[high.len() - MAX_PATTERN_LENGTH - 1..].to_vec(),
            low: low[low.len() - MAX_PATTERN_LENGTH - 1..].to_vec(),
            close: close[close.len() - MAX_PATTERN_LENGTH - 1..].to_vec(),
        }
    }
    /// Runs the Candlestick Pattern indicator incrementally on a new batch of bars.
    ///
    /// # Inputs
    ///
    /// * `inputs[0]` — `open`
    /// * `inputs[1]` — `high`
    /// * `inputs[2]` — `low`
    /// * `inputs[3]` — `close`
    ///
    /// # Arguments
    ///
    /// * `inputs` - Array of input price slices (see Inputs above).
    /// * `forecast_type` - Pass `Some(ForecastType::…)` to filter by forecast direction,
    ///   or `None` to return all detected patterns.
    ///
    /// # Returns
    ///
    /// `Ok(output)` where `output[i]` is `Some(patterns)` when one or more patterns
    /// are detected on that bar, or `None` otherwise.
    /// Returns `Err(IndicatorError)` if the input slices are empty.
    pub fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        forecast_type: Option<ForecastType>,
    ) -> Result<Vec<Option<Vec<CandlePattern>>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.open.extend_from_slice(inputs[0]);
        self.high.extend_from_slice(inputs[1]);
        self.low.extend_from_slice(inputs[2]);
        self.close.extend_from_slice(inputs[3]);

        let capacity = inputs[0].len();

        let mut output = vec![None; capacity];

        cycle(
            &self.open,
            &self.high,
            &self.low,
            &self.close,
            MAX_PATTERN_LENGTH + 1,
            &mut self.state,
            &mut output,
            forecast_type,
        );

        self.open.drain(..self.open.len() - MAX_PATTERN_LENGTH - 1);
        Ok(output)
    }
}

/// Calculates the Candlestick Pattern indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `open`
/// * `inputs[1]` — `high`
/// * `inputs[2]` — `low`
/// * `inputs[3]` — `close`
///
/// # Options
///
/// * `options[0]` — `candle_period` (EMA period for candle body averages)
/// * `options[1]` — `trend_period` (EMA period for trend detection)
/// * `options[2]` — `trend_signal_period` (EMA period for the trend signal line)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `forecast_type` - Pass `Some(ForecastType::…)` to filter detected patterns by
///   forecast direction, or `None` to return all patterns.
///
/// # Returns
///
/// `Ok((output, state))` where each element of `output` is `Some(patterns)` when
/// one or more patterns are detected on that bar, or `None` otherwise, and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    forecast_type: Option<ForecastType>,
) -> Result<(Vec<Option<Vec<CandlePattern>>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let candle_period = options[0] as usize;
    let trend_period = options[1] as usize;
    let signal_period = options[2] as usize;

    let mut state = State::init(inputs, candle_period, trend_period, signal_period);

    let greater_period = if candle_period > trend_period {
        candle_period
    } else {
        trend_period
    };

    let (open, high, low, close) = (
        &inputs[0][greater_period..],
        &inputs[1][greater_period..],
        &inputs[2][greater_period..],
        &inputs[3][greater_period..],
    );

    let capacity = output_length(inputs[0].len(), options);

    let mut output = vec![None; capacity];

    // Process each candle
    cycle(
        open,
        high,
        low,
        close,
        open.len() - output.len(),
        &mut state,
        &mut output,
        forecast_type,
    );

    Ok((output, IndicatorState::new(state, open, high, low, close)))
}

/// Iterates over the OHLC slices (starting at `start`) and writes candlestick
/// pattern results into `output`.
///
/// # Arguments
///
/// * `open` - Input open price slice.
/// * `high` - Input high price slice.
/// * `low` - Input low price slice.
/// * `close` - Input close price slice.
/// * `start` - The index within the slices at which to begin emitting output.
/// * `state` - Mutable reference to the pattern-detection `State`.
/// * `output` - Mutable output vector; one entry per bar from `start` onward.
/// * `forecast_type` - An optional `ForecastType` controlling which patterns are detected.
fn cycle(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    start: usize,
    state: &mut State,
    output: &mut Vec<Option<Vec<CandlePattern>>>,
    forecast_type: Option<ForecastType>,
) {
    for (j, i) in (start..open.len()).enumerate() {
        let patterns = state.calc(open, high, low, close, i, forecast_type);
        unsafe { *output.get_unchecked_mut(j) = patterns };
    }
}

/// Returns EMA multiplier tuples for the three candlestick periods.
///
/// # Arguments
///
/// * `candle_period` - The period for the candle body EMA.
/// * `trend_period` - The period for the trend EMA.
/// * `trend_signal_period` - The period for the trend signal EMA.
///
/// # Returns
///
/// A tuple of three `(multiplier, complement)` pairs, one for each period.
pub fn multiplier(
    candle_period: usize,
    trend_period: usize,
    trend_signal_period: usize,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    (
        ema_multiplier(candle_period),
        ema_multiplier(trend_period),
        ema_multiplier(trend_signal_period),
    )
}
