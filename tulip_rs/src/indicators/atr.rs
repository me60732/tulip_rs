use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::tr::{calc as calc_tr, output_length as tr_output_length};
pub use crate::indicators::wilders::multiplier;
use crate::indicators::wilders::{calc as calc_wilders, partial_calc as partial_calc_wilders};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::atr_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::atr_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::atr_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::atr_simd::indicator_by_options as indicator;
}

/// Returns information about the Average True Range (ATR) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the ATR indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "atr",
        full_name: "Average True Range",
        indicator_type: IndicatorType::Volatility,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["atr"],
        optional_outputs: &["tr"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub atr: f64,
    pub prev_close: f64,
    multiplier: f64,
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
}
impl IndicatorState {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}
impl State {
    pub fn new(atr: f64, prev_close: f64, multiplier: f64) -> Self {
        Self {
            atr,
            prev_close,
            multiplier,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
        composite: bool,
    ) -> State {
        let mut atr = high[0] - low[0]; //if !composite { high[0] - low[0] } else { 0.0 };
        let mut tr;
        if period < high.len() {
            for (i, (&h, &l)) in high.iter().zip(low.iter()).enumerate().take(period).skip(1) {
                let prev_close = unsafe { *close.get_unchecked(i - 1) };
                (atr, tr) = init_calc(h, l, prev_close, atr);

                if tr_line.len() > 0 {
                    tr_line[i - 1] = tr;
                }
            }
        }
        if !composite {
            atr /= period as f64;
        }
        State {
            atr,
            prev_close: close[period - 1],
            multiplier: multiplier(period).0,
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64) -> (f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        self.atr = calc_wilders(self.atr, tr, self.multiplier);
        self.prev_close = close;
        (self.atr, tr)
    }
    #[inline(always)]
    pub fn partial_calc(&mut self, high: f64, low: f64, close: f64) -> (f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        self.atr = partial_calc_wilders(self.atr, tr, self.multiplier);
        self.prev_close = close;
        (self.atr, tr)
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut atr_line = crate::uninit_vec!(f64, inputs[0].len());

        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: inputs[0].len()
        );
        cycle_atr(
            (inputs[0], inputs[1], inputs[2]),
            &mut self.state,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![atr_line, tr_line])
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).1],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the ATR indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the ATR calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1 // period
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the ATR calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Average True Range (ATR) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `options` - A slice containing the period for the ATR calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the ATR line and any additional requested outputs.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut atr_line = {
        let atr_capacity = output_length(high.len(), options);
        crate::uninit_vec!(f64, atr_capacity)
    };

    let mut tr_line = crate::init_optional_outputs_eff!(
        optional_outputs, &[false],
        tr_line: tr_output_length(high.len(), options)
    );
    let mut state = State::init_state(high, low, close, period, &mut tr_line, false);
    let tr_offset = crate::slice_outputs_start!(atr_line.len(), tr_line);
    cycle_atr(
        (&high[period..], &low[period..], &close[period..]),
        &mut state,
        (&mut atr_line, &mut tr_line[tr_offset..]),
    );

    Ok((vec![atr_line, tr_line], IndicatorState { state: state }))
}

/// Performs the main calculation loop for the ATR indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `period` - The period for the ATR calculation.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
/// * `atr_line` - A mutable reference to a vector for storing the ATR line.
/// * `output_vectors` - A mutable reference to an array of optional vectors for storing additional outputs.
fn cycle_atr(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State,
    outputs: (&mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (atr_line, tr_line) = outputs;
    let (_, want_tr) = crate::calc_want_flags!(tr_line);

    for i in 0..high.len() {
        let (atr, tr);
        unsafe {
            let (h, l, c) = (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
            (atr, tr) = calc(state, h, l, c);
            *atr_line.get_unchecked_mut(i) = atr;
        }
        crate::store_optional_outputs!(i,
            want_tr, tr_line => tr
        );
    }
}
/// Calculates the current value of the Average True Range (ATR) indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple containing the high and low prices.
/// * `prev_close` - The previous close price.
/// * `prev_atr` - The previous ATR value.
/// * `period` - The period for the ATR calculation.
///
/// # Returns
///
/// The updated ATR value.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64) {
    state.calc(high, low, close)
}
#[inline(always)]
pub fn partial_calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64) {
    state.partial_calc(high, low, close)
}

#[inline(always)]
pub(crate) fn init_calc(high: f64, low: f64, prev_close: f64, atr: f64) -> (f64, f64) {
    let tr = calc_tr(high, low, prev_close);
    (atr + tr, tr)
}

/*#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64) {
    wilders_multiplier(period)
}*/
