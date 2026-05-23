use crate::candle_indicators::candle_patterns::*;
use crate::candle_indicators::pattern_test::{State, MAX_PATTERN_LENGTH};
pub use crate::candle_indicators::types::ForecastType;
use crate::common::{validate_inputs, validate_options};
use crate::indicators::ema::{min_data as ema_min_data, multiplier as ema_multiplier};
use serde::{Deserialize, Serialize};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
pub const INPUTS_WIDTH: usize = 4;
pub const OPTIONS_WIDTH: usize = 3;

pub fn info() -> Info<'static> {
    Info {
        name: "candlestick",
        full_name: "Candle Stick Indicator",
        indicator_type: IndicatorType::CandleStick,
        display_type: DisplayType::Overlay,
        inputs: &["open", "high", "low", "close"],
        options: &["candle_period", "trend_period", "trend_signal_period"],
        outputs: &["cdl_pattern"],
        optional_outputs: &[],
    }
}

pub fn min_data(options: &[f64]) -> usize {
    if options[0] > options[1] + options[2] {
        return ema_min_data(&[options[0]]) + MAX_PATTERN_LENGTH;
    }
    ema_min_data(&[options[1]]) + options[2] as usize + MAX_PATTERN_LENGTH + 1
}
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

        cycle(&self.open, &self.high, &self.low, &self.close, MAX_PATTERN_LENGTH + 1, &mut self.state, &mut output, forecast_type);

        self.open.drain(..self.open.len() - MAX_PATTERN_LENGTH - 1);
        Ok(output)
    }
}

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

    let mut state = State::init(
        inputs,
        candle_period,
        trend_period,
        signal_period,
    );

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
    
    Ok((
        output, 
        IndicatorState::new(state, open, high, low, close)
    ))
}

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
