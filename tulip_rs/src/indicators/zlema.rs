use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::ema::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    real: Vec<f64>,
    lag: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, lag: usize) -> Self {
        Self {
            state,
            real: real[real.len() - lag..].to_vec(),
            lag,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub zlema: f64,
    pub per: f64,
    pub multiplier: f64,
}
impl State {
    pub fn new(real: &[f64], lag: usize, period: usize) -> Self {
        let (multiplier, per) = multiplier(period);
        Self {
            zlema: real[lag - 1],
            multiplier,
            per,
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, current: f64, lagged: f64) -> f64 {
        let adjusted = current + (current - lagged);

        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        self.zlema
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        // Merge stored trailing real values with new input.
        self.real.extend_from_slice(inputs[0]);

        let mut zlema_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_zlema(&self.real, self.lag, &mut self.state, &mut zlema_line);

        self.real.drain(..self.real.len() - self.lag);

        Ok(vec![zlema_line])
    }
}
pub fn info() -> Info<'static> {
    Info {
        name: "zlema",
        full_name: "Zero Lag Exponential Moving Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        // One input: real (can be any price series).
        inputs: &["real"],
        // One option: period.
        options: &["period"],
        outputs: &["zlema"],
        optional_outputs: &[],
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
/// * `options` - A slice containing the indicator options: `[period]`.
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the ZLEMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[period]`.
///
/// # Returns
///
/// The minimum amount of data required (derived from the lag: `(period - 1) / 2 + 1`).
pub fn min_data(options: &[f64]) -> usize {
    ((options[0] as usize - 1) / 2) + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the ZLEMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Zero Lag Exponential Moving Average (ZLEMA) indicator over the full input dataset.
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
/// `Ok((outputs, state))` where `outputs[0]` is `zlema` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let lag = ((period.saturating_sub(1)) / 2).max(1);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut zlema_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::new(real, lag, period);

    cycle_zlema(real, lag, &mut state, &mut zlema_line);

    Ok((vec![zlema_line], IndicatorState::new(real, state, lag)))
}

/// Iterates over the real data slice and computes ZLEMA values for each bar.
///
/// # Arguments
///
/// * `real` - The full input data slice (includes the leading lag values).
/// * `lag` - The number of look-back bars used for zero-lag adjustment.
/// * `state` - Mutable reference to the rolling `State` (previous ZLEMA, multipliers).
/// * `zlema_line` - Mutable output slice for ZLEMA values.
fn cycle_zlema(real: &[f64], lag: usize, state: &mut State, zlema_line: &mut [f64]) {
    for (j, i) in (lag..real.len()).enumerate() {
        unsafe {
            *zlema_line.get_unchecked_mut(j) =
                state.calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Calculates a single ZLEMA value for one bar, updating the rolling state in place.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State` (previous ZLEMA, multipliers).
/// * `current` - The current input value.
/// * `lagged` - The input value from `lag` bars ago.
///
/// # Returns
///
/// The updated ZLEMA value for this bar.
#[inline(always)]
pub fn calc(state: &mut State, current: f64, lagged: f64) -> f64 {
    state.calc(current, lagged)
}
