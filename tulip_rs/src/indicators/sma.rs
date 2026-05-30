use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::sma_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::sma_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::sma_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::sma_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    //state: State,
    multiplier: f64,
    sum: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, multiplier: f64, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            //state: State::new(sum, multiplier),
            sum,
            period,
            multiplier,
        }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    /// Continues the Simple Moving Average (SMA) calculation from the stored state.
    ///
    /// # Arguments
    ///
    /// * `inputs` - An array of one input slice: `[real]`.
    /// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
    ///
    /// # Returns
    ///
    /// `Result<Vec<Vec<f64>>, IndicatorError>` — a vector of output vectors containing the SMA line.
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let mut sma_line: Vec<f64> = crate::uninit_vec!(f64, inputs[0].len());
        self.real.extend_from_slice(inputs[0]);
        cycle_sma(
            &self.real,
            self.period,
            &mut sma_line,
            &mut self.sum,
            &self.multiplier,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![sma_line])
    }
}
/// Returns information about the Simple Moving Average (SMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the SMA indicator.
pub const INFO: Info = Info {
    name: "sma",
    full_name: "Simple Moving Average",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period"],
    outputs: &["sma"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "sma",
        label: "SMA",
        display_type: DisplayType::Overlay,
        outputs: &["sma"],
    }],
};
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
/// Returns the minimum amount of data required for the SMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the SMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}
/// Calculates the output length for the SMA indicator given the input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the SMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub fn init_state(real: &[f64], period: usize) -> f64 {
    let mut sum = 0.0;
    for i in 0..period {
        sum += real[i];
    }
    sum
}

/// Calculates the Simple Moving Average (SMA) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (source) values
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `sma`
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
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
    let mut sum = init_state(real, period);
    let multiplier = multiplier(period);
    let mut sma_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle_sma(real, period, &mut sma_line, &mut sum, &multiplier);

    Ok((
        vec![sma_line],
        IndicatorState::new(real, sum, multiplier, period),
    ))
}

/// Performs the main calculation loop for the SMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the SMA calculation.
/// * `sma_line` - A mutable slice for storing the SMA output values.
/// * `sum` - A mutable reference to the running sum of the window values.
/// * `multiplier` - A reference to the precomputed multiplier (1/period).
fn cycle_sma(real: &[f64], period: usize, sma_line: &mut [f64], sum: &mut f64, multiplier: &f64) {
    //let multiplier = &multiplier(period);
    for (j, i) in (period..real.len()).enumerate() {
        let sma = unsafe {
            calc(
                sum,
                real.get_unchecked(i),
                real.get_unchecked(j),
                multiplier,
            )
        };
        unsafe { *sma_line.get_unchecked_mut(j) = sma };
    }
}
/// Calculates the current value of the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `sum` - A mutable reference to the running sum of the window values.
/// * `value` - The current input value entering the window.
/// * `prev_value` - The oldest input value leaving the window.
/// * `multiplier` - A reference to the precomputed multiplier (1/period).
///
/// # Returns
///
/// The current SMA value.
#[inline(always)]
pub fn calc(sum: &mut f64, value: &f64, prev_value: &f64, multiplier: &f64) -> f64 {
    let mut s = *sum;
    s = s + (value - prev_value);
    *sum = s;
    s * multiplier
}
/// Calculates the multiplier for the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `period` - The period for the SMA calculation.
///
/// # Returns
///
/// The multiplier for the SMA calculation.
#[inline(always)]
pub fn multiplier(period: usize) -> f64 {
    1.0 / period as f64
}
