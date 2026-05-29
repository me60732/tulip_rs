use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::sma::init_state;
pub use crate::indicators::sma::multiplier;
use crate::indicators::sma::{calc as calc_sma, output_length as sma_output_length};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    multiplier: f64,
    sum: f64,
    dpo_period: usize,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, multiplier: f64, period: usize, dpo_period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            sum,
            multiplier,
            period,
            dpo_period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut dpo_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_dpo(
            &self.real,
            (self.period, self.dpo_period),
            self.multiplier,
            &mut self.sum,
            &mut dpo_line,
            &mut sma_line,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![dpo_line, sma_line])
    }
}
/// Returns information about the Detrended Price Oscillator (DPO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DPO indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "dpo",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Cycle,
        full_name: "Detrended Price Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["dpo"],
        optional_outputs: &["sma"],
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
/// Returns the minimum amount of data required for the DPO indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DPO calculation.
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
/// * `options` - A slice containing the options for the DPO calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Detrended Price Oscillator (DPO) over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real values (typically close prices)
///
/// # Options
///
/// * `options[0]` — period (SMA window length; look-back offset is `period / 2 + 1`)
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to enable the optional `sma`
///   output; `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `dpo` and `outputs[1]` is `sma`
/// (empty unless requested). `state` can be passed to `IndicatorState::batch_indicator`
/// for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let dpo_period = period / 2 + 1;

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut dpo_line, mut sma_line) = {
        let sma_capacity = sma_output_length(real.len(), options);
        let capacity = output_length(real.len(), options);

        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                sma_line: sma_capacity
            ),
        )
    };

    let mut sum = init_state(real, period);
    let multiplier = multiplier(period);
    // Perform the main DPO calculation
    cycle_dpo(
        real,
        (period, dpo_period),
        multiplier,
        &mut sum,
        &mut dpo_line,
        &mut sma_line,
    );

    Ok((
        vec![dpo_line, sma_line],
        IndicatorState {
            sum,
            period,
            dpo_period,
            multiplier,
            real: real[real.len() - period..].to_vec(),
        },
    ))
}

/// Performs the main calculation loop for the DPO indicator.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `periods` - A tuple `(period, dpo_period)` where `period` is the SMA window and
///   `dpo_period` is the look-back offset used to detrend the price.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
/// * `sum` - Mutable reference to the running sum used for the SMA calculation.
/// * `dpo_line` - Mutable slice to write the DPO output values into.
/// * `sma_line` - Mutable slice to write the SMA values into (optional output).
fn cycle_dpo(
    real: &[f64],
    periods: (usize, usize),
    multiplier: f64,
    sum: &mut f64,
    dpo_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (period, dpo_period) = periods;
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (value, prev_values);
        unsafe {
            value = real.get_unchecked(i);
            prev_values = (real.get_unchecked(j), real.get_unchecked(i - dpo_period));
        }
        let (dpo, sma) = calc(value, sum, prev_values, multiplier);
        unsafe {
            *dpo_line.get_unchecked_mut(j) = dpo;
        }
        crate::store_optional_outputs!(j, want_sma, sma_line => sma);
    }
}

/// Calculates the DPO and SMA values for the current data point.
///
/// # Arguments
///
/// * `value` - The current input value.
/// * `sum` - Mutable reference to the running sum used for the SMA calculation.
/// * `prev_values` - A tuple `(prev_value, dpo_price)` where `prev_value` is the value
///   leaving the SMA window and `dpo_price` is the historical price used for detrending.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
///
/// # Returns
///
/// A tuple `(dpo, sma)` representing the Detrended Price Oscillator and the current SMA.
#[inline(always)]
pub fn calc(value: &f64, sum: &mut f64, prev_values: (&f64, &f64), multiplier: f64) -> (f64, f64) {
    //let (sma, mut s) = (0.0, *sum);
    let (prev_value, dpo_price) = prev_values;
    let sma = calc_sma(sum, value, prev_value, &multiplier);
    (dpo_price - sma, sma)
}
