use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dm_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dm_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::dm_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::dm_simd::indicator_by_options as indicator;
}
/// Returns information about the Directional Movement (DM) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DM indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "dm",
        full_name: "Directional Movement",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["plus_dm", "minus_dm"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub dmup: f64,
    pub dmdown: f64,
    multiplier: f64,
    pub prev_high: f64,
    pub prev_low: f64,
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
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut plus_dm_line, mut minus_dm_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let [high, low] = inputs;
        cycle_calc(
            high,
            low,
            &mut self.state,
            &mut plus_dm_line,
            &mut minus_dm_line,
        );

        Ok(vec![plus_dm_line, minus_dm_line])
    }
}
impl State {
    pub fn new(dmup: f64, dmdown: f64, prev_high: f64, prev_low: f64, multiplier: f64) -> Self {
        Self {
            dmup,
            dmdown,
            prev_high,
            prev_low,
            multiplier,
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> State {
        let mut state = State::new(0.0, 0.0, high[0], low[0], multiplier(period));
        for (&h, &l) in high.iter().zip(low.iter()).take(period).skip(1) {
            let (dp, dm) = calc_dp_dm(&mut state, h, l);
            state.dmup += dp;
            state.dmdown += dm;
        }
        state
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[1.0 - multiplier(options[0] as usize)],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the DM indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DM calculation.
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
/// * `options` - A slice containing the options for the DM calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Directional Movement (DM) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high and low prices.
/// * `options` - A slice containing the period for the DM calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the plus DM line and minus DM line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    let (mut plus_dm_line, mut minus_dm_line) = {
        let capacity: usize = output_length(inputs[0].len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        )
    };

    let mut state = State::init_state(inputs[0], inputs[1], period);
    let (high, low) = (&inputs[0][period..], &inputs[1][period..]);
    cycle_calc(high, low, &mut state, &mut plus_dm_line, &mut minus_dm_line);

    Ok((
        vec![plus_dm_line, minus_dm_line],
        IndicatorState { state: state },
    ))
}
/// Calculates the Directional Movement (DM) indicator, picking up where the previous calculation left off.
///
/// This function is useful for scenarios where indicator data is stored in a database and you need to continue calculations from the last stored state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high and low prices.
/// * `options` - A slice containing the period for the DM calculation.
/// * `indicator_state` - An `IndicatorState` struct containing necessary input values.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the plus DM line and minus DM line.

/// Performs the main calculation loop for the DM indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `period` - The period for the DM calculation.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
/// * `capacity` - The capacity of the output vectors.
///
/// # Returns
///
/// A tuple containing the plus DM line and minus DM line.
fn cycle_calc(
    high: &[f64],
    low: &[f64],
    state: &mut State,
    plus_dm_line: &mut [f64],
    minus_dm_line: &mut [f64],
) {
    for i in 0..high.len() {
        unsafe {
            let (h, l) = (*high.get_unchecked(i), *low.get_unchecked(i));
            let (dmup, dmdown) = calc(state, h, l);
            *plus_dm_line.get_unchecked_mut(i) = dmup;
            *minus_dm_line.get_unchecked_mut(i) = dmdown;
        }
    }
}

/// Calculates the current DM+ and DM- values.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `prev_high` - The previous high price.
/// * `prev_low` - The previous low price.
///
/// # Returns
///
/// A tuple containing the current DM+ value and the current DM- value.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64) -> (f64, f64) {
    let (dp, dm) = calc_dp_dm(state, high, low);
    let (_, _) = calc_dmup_dmdown(state, dp, dm);
    (state.dmup, state.dmdown)
}

/// Calculates the updated DM+ and DM- values using the Wilder's smoothing method.
///
/// # Arguments
///
/// * `dp` - The current DM+ value.
/// * `dm` - The current DM- value.
/// * `dmup` - The previous DM+ value.
/// * `dmdown` - The previous DM- value.
/// * `period` - The period for the DM calculation.
///
/// # Returns
///
/// A tuple containing the updated DM+ value and the updated DM- value.
#[inline(always)]
fn calc_dmup_dmdown(state: &mut State, dp: f64, dm: f64) -> (f64, f64) {
    //state.dmup = state.multiplier * state.dmup + dp;
    state.dmup = state.dmup.mul_add(state.multiplier, dp);
    //state.dmdown = state.multiplier * state.dmdown + dm;
    state.dmdown = state.dmdown.mul_add(state.multiplier, dm);
    (state.dmup, state.dmdown)
}
/// Calculates the current DM+ and DM- values.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `prev_high` - The previous high price.
/// * `low` - The current low price.
/// * `prev_low` - The previous low price.
///
/// # Returns
///
/// A tuple containing the current DM+ value and the current DM- value.
#[inline(always)]
pub fn calc_dp_dm(state: &mut State, high: f64, low: f64) -> (f64, f64) {
    let mut dp = high - state.prev_high; //.max(0.0);
    let mut dm = state.prev_low - low; //.max(0.0);
    (state.prev_high, state.prev_low) = (high, low);

    if dp < 0.0 {
        dp = 0.0;
    } else if dp > dm {
        dm = 0.0;
    }

    if dm < 0.0 {
        dm = 0.0;
    } else if dm > dp {
        dp = 0.0;
    }

    (dp, dm)
}
#[inline]
pub fn multiplier(period: usize) -> f64 {
    ((period - 1) as f64) / period as f64
}
