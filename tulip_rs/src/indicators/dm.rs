use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dm_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dm_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dm_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
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
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the Wilder's smoothing
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
/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the DM calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Directional Movement (DM) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period (Wilder smoothing window for DM+ / DM-)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; DM has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `plus_dm`, `outputs[1]` is `minus_dm`,
/// and `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
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
/// Performs the main calculation loop for the DM indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `state` - Mutable reference to the DM state.
/// * `plus_dm_line` - Mutable slice to write the DM+ output values into.
/// * `minus_dm_line` - Mutable slice to write the DM- output values into.
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

/// Calculates the smoothed DM+ and DM- values for the current bar.
///
/// # Arguments
///
/// * `state` - Mutable reference to the DM state.
/// * `high` - The current high price.
/// * `low` - The current low price.
///
/// # Returns
///
/// A tuple `(plus_dm, minus_dm)` of the smoothed directional movement values.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64) -> (f64, f64) {
    let (dp, dm) = calc_dp_dm(state, high, low);
    let (_, _) = calc_dmup_dmdown(state, dp, dm);
    (state.dmup, state.dmdown)
}

/// Applies Wilder's smoothing to update DM+ and DM- in state.
///
/// # Arguments
///
/// * `state` - Mutable reference to the DM state containing `dmup`, `dmdown`, and `multiplier`.
/// * `dp` - The raw DM+ value for the current bar.
/// * `dm` - The raw DM- value for the current bar.
///
/// # Returns
///
/// A tuple `(dmup, dmdown)` of the updated smoothed directional movement values.
#[inline(always)]
fn calc_dmup_dmdown(state: &mut State, dp: f64, dm: f64) -> (f64, f64) {
    //state.dmup = state.multiplier * state.dmup + dp;
    state.dmup = state.dmup.mul_add(state.multiplier, dp);
    //state.dmdown = state.multiplier * state.dmdown + dm;
    state.dmdown = state.dmdown.mul_add(state.multiplier, dm);
    (state.dmup, state.dmdown)
}
/// Calculates the raw DM+ and DM- values for the current bar.
///
/// Uses `state.prev_high` and `state.prev_low` as the previous bar's values,
/// then updates them to `high` and `low`.
///
/// # Arguments
///
/// * `state` - Mutable reference to the DM state (reads and updates `prev_high` and `prev_low`).
/// * `high` - The current high price.
/// * `low` - The current low price.
///
/// # Returns
///
/// A tuple `(dp, dm)` of the raw directional movement values before smoothing.
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
