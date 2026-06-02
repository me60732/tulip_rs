use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::tr::{calc as calc_tr, output_length as tr_output_length};
pub use crate::indicators::wilders::multiplier;
use crate::indicators::wilders::{calc as calc_wilders, partial_calc as partial_calc_wilders};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::atr_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::atr_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::atr_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::atr_simd::indicator_by_options as indicator;
}

/// Returns information about the Average True Range (ATR) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the ATR indicator.
pub const INFO: Info = Info {
    name: "atr",
    full_name: "Average True Range",
    indicator_type: IndicatorType::Volatility,
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["atr"],
    optional_outputs: &["tr"],
    display_groups: &[
        DisplayGroup {
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        }
    ],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub atr: f64,
    pub prev_close: f64,
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64)) -> Self {
        Self { state, multipliers }
    }
}
impl State {
    pub fn new(atr: f64, prev_close: f64) -> Self {
        Self {
            atr,
            prev_close,
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
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64, multipliers: (f64, f64)) -> (f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        self.atr = calc_wilders(self.atr, tr, multipliers);
        self.prev_close = close;
        (self.atr, tr)
    }
    #[inline(always)]
    pub fn partial_calc(&mut self, high: f64, low: f64, close: f64, multipliers: (f64, f64)) -> (f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        self.atr = partial_calc_wilders(self.atr, tr, multipliers.0);
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
            self.multipliers,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![atr_line, tr_line])
    }
}
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the smoothing
/// multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
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
/// Calculates the output length for the ATR indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the ATR calculation.
///
/// # Returns
///
/// The number of output values produced by the ATR calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Average True Range (ATR) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to also emit the true range (`tr`) line;
///   `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `atr`, `outputs[1]` is `tr` (optional),
/// and `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
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
    let multipliers = multiplier(period);
    let mut tr_line = crate::init_optional_outputs_eff!(
        optional_outputs, &[false],
        tr_line: tr_output_length(high.len(), options)
    );
    let mut state = State::init_state(high, low, close, period, &mut tr_line, false);
    let tr_offset = crate::slice_outputs_start!(atr_line.len(), tr_line);
    cycle_atr(
        (&high[period..], &low[period..], &close[period..]),
        &mut state,
        multipliers,
        (&mut atr_line, &mut tr_line[tr_offset..]),
    );

    Ok((vec![atr_line, tr_line], IndicatorState { state: state, multipliers: multipliers }))
}

/// Performs the main calculation loop for the ATR indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high, low, and close price slices.
/// * `state` - A mutable reference to the current ATR state.
/// * `outputs` - A tuple of mutable slices for storing the ATR line and optional TR line.
fn cycle_atr(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State,
    multipliers: (f64, f64),
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
            (atr, tr) = calc(state, h, l, c, multipliers);
            *atr_line.get_unchecked_mut(i) = atr;
        }
        crate::store_optional_outputs!(i,
            want_tr, tr_line => tr
        );
    }
}
/// Calculates the current ATR and true range values.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current ATR state.
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
///
/// # Returns
///
/// A tuple of `(atr, tr)` containing the updated ATR value and current true range.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64, multipliers: (f64, f64)) -> (f64, f64) {
    state.calc(high, low, close, multipliers)
}
#[inline(always)]
pub fn partial_calc(state: &mut State, high: f64, low: f64, close: f64, multipliers: (f64, f64)) -> (f64, f64) {
    state.partial_calc(high, low, close, multipliers)
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
