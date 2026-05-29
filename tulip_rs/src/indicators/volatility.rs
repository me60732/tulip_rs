use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::stddev::multiplier;
use crate::indicators::stddev::{calc as stddev_calc, State as StddevState};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options as indicator;
}
const ANNUAL: f64 = 15.874507866387544; // 252_f64.sqrt()

pub fn info() -> Info<'static> {
    Info {
        name: "volatility",
        full_name: "Volatility Indicator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Volatility,
        inputs: &["real"],
        options: &["period"],
        outputs: &["volatility"],
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

        let mut volatility_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            self.multiplier,
            &mut self.state,
            &mut volatility_line,
        );

        Ok(vec![volatility_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub stddev_state: StddevState,
    pub prev_real: f64,
}
impl State {
    pub fn new(prev_real: f64, period: usize) -> Self {
        let stddev_state = StddevState::new(0.0, 0.0);
        let buffer = Buffer::new(period);
        State {
            prev_real,
            stddev_state,
            buffer,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let (mut sum, mut sum_sq) = (0.0, 0.0);
        let mut buffer = Buffer::new(period);
        for i in 1..=period {
            let v = real[i] / real[i - 1] - 1.0;
            buffer.push(v);
            sum += v;
            sum_sq += v * v;
        }

        Self {
            stddev_state: StddevState::new(sum, sum_sq),
            buffer,
            prev_real: real[period],
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, real: f64, multiplier: f64) -> f64 {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info(value).unwrap();
        let (sd, _) = stddev_calc(&mut self.stddev_state, &value, &prev_value, multiplier);
        sd * ANNUAL
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, real: f64, multiplier: f64) -> f64 {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info_unchecked(value);
        let (sd, _) = stddev_calc(&mut self.stddev_state, &value, &prev_value, multiplier);
        sd * ANNUAL
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options: `[period]`.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Volatility indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[period]`.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 2
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Volatility calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Volatility indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `real` (price series)
///
/// # Options
///
/// * `options[0]` — `period`
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `volatility` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let mut vol_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };
    let mut state = State::init_state(inputs[0], period);

    cycle(
        &inputs[0][period + 1..],
        multiplier,
        &mut state,
        &mut vol_line,
    );

    Ok((vec![vol_line], IndicatorState { multiplier, state }))
}
/// Iterates over the real data slice and computes a Volatility value for each bar.
///
/// # Arguments
///
/// * `real` - Input data slice starting after the initialization window.
/// * `multiplier` - The stddev multiplier computed from the period.
/// * `state` - Mutable reference to the rolling calculation state.
/// * `vol_line` - Mutable output slice for volatility values.
fn cycle(real: &[f64], multiplier: f64, state: &mut State, vol_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *vol_line.get_unchecked_mut(i) =
                state.calc_unchecked(*real.get_unchecked(i), multiplier);
        }
    }
}

/// Calculates a single Volatility value for one bar, updating the rolling state.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State`.
/// * `real` - The current input value.
/// * `multiplier` - The stddev multiplier computed from the period.
///
/// # Returns
///
/// The annualised volatility value for this bar.
#[inline(always)]
pub fn calc(state: &mut State, real: f64, multiplier: f64) -> f64 {
    state.calc(real, multiplier)
}
/// Calculates a single Volatility value for one bar using unchecked buffer access.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State`.
/// * `real` - The current input value.
/// * `multiplier` - The stddev multiplier computed from the period.
///
/// # Returns
///
/// The annualised volatility value for this bar.
///
/// # Safety
///
/// The internal ring buffer must have been fully initialised before calling this function.
#[inline(always)]
pub unsafe fn calc_unchecked(state: &mut State, real: f64, multiplier: f64) -> f64 {
    state.calc_unchecked(real, multiplier)
}
