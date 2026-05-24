use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::cmo_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::cmo_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::cmo_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::cmo_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, period: usize) -> Self {
        Self {
            real: real[real.len() - period - 1..].to_vec(),
            state,
            period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut cmo_line = crate::uninit_vec!(f64, inputs[0].len());

        self.real.extend_from_slice(inputs[0]);

        //let mut cmo_line: Vec<f64> = vec![0.0; capacity];

        cycle_cmo(&self.real, &mut self.state, self.period, &mut cmo_line);

        self.real.drain(..self.real.len() - self.period - 1);

        Ok(vec![cmo_line])
    }
}
/// Returns information about the Chande Momentum Oscillator (CMO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the CMO indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "cmo",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        full_name: "Chande Momentum Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["cmo"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub up_sum: f64,
    pub down_sum: f64,
}
impl State {
    pub fn new(up_sum: f64, down_sum: f64) -> Self {
        State { up_sum, down_sum }
    }
    /// Calculates the initial up and down sums for the CMO calculation.
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let (up, down) = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        Self::new(up_sum, down_sum)
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the CMO indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the CMO calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 2
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the CMO calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
#[inline(always)]
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Chande Momentum Oscillator (CMO) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real values (typically close prices)
///
/// # Options
///
/// * `options[0]` — period (look-back window for the momentum calculation)
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; CMO has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `cmo` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut cmo_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(real, period);
    cycle_cmo(real, &mut state, period, &mut cmo_line);

    Ok((vec![cmo_line], IndicatorState::new(real, state, period)))
}

/// Performs the main calculation loop for the CMO indicator.
///
/// # Arguments
///
/// * `real` - A slice of real values.
/// * `state` - Mutable reference to the CMO state (running up and down sums).
/// * `period` - The period for the CMO calculation.
/// * `cmo_line` - Mutable slice to write the CMO output values into.
fn cycle_cmo(real: &[f64], state: &mut State, period: usize, cmo_line: &mut [f64]) {
    for (j, i) in (period + 1..real.len()).enumerate() {
        unsafe {
            let prev_before = *real.get_unchecked(j);
            let prev_period = *real.get_unchecked(j + 1);
            let prev = *real.get_unchecked(i - 1);
            let current = *real.get_unchecked(i);

            let cmo = calc(state, prev_before, prev_period, current, prev);

            *cmo_line.get_unchecked_mut(j) = cmo;
        }
    }
}

#[inline(always)]
pub fn up_down(value: f64, prev_value: f64) -> (f64, f64) {
    let diff = value - prev_value;
    (diff.max(0.0), (-diff).max(0.0))
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    prev_real_0: f64,
    prev_real_1: f64,
    cur_real: f64,
    prior_real: f64,
) -> f64 {
    let (old_up, old_down) = up_down(prev_real_1, prev_real_0);
    let (up, down) = up_down(cur_real, prior_real);
    state.up_sum += up - old_up;
    state.down_sum += down - old_down;

    100.0 * (state.up_sum - state.down_sum) / (state.up_sum + state.down_sum)
}
