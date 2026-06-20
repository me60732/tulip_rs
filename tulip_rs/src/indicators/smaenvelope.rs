use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::sma::init_state;
use crate::indicators::sma::{calc as calc_sma, multiplier as sma_multiplier};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

// SIMD variants are not yet implemented for SMA Envelope.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_options as indicator;
}

/// Returns static metadata about the SMA Envelope indicator.
///
/// # Returns
///
/// An `Info` struct containing the name, type, input/option/output descriptions,
/// and display group configuration for the SMA Envelope indicator.
pub const INFO: Info = Info {
    name: "smaenvelope",
    full_name: "SMA Envelope",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period", "percentage"],
    outputs: &["lower", "middle", "upper"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "smaenvelope",
        label: "SMA Envelope",
        display_type: DisplayType::Overlay,
        outputs: &["lower", "middle", "upper"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    sum: f64,
    period: usize,
    multipliers: (f64, f64),
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, period: usize, multipliers: (f64, f64)) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            sum,
            period,
            multipliers,
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

        self.real.extend_from_slice(inputs[0]);

        let (mut middle_band, mut upper_band, mut lower_band) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        cycle(
            &self.real,
            self.period,
            self.multipliers,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut self.sum,
        );

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![lower_band, middle_band, upper_band])
    }
}
/// Returns the minimum number of input bars required for the SMA Envelope indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options. `options[0]` is the period.
///
/// # Returns
///
/// `period + 1` — the smallest number of bars needed to emit at least one output value.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the number of output values produced by the SMA Envelope indicator.
///
/// # Arguments
///
/// * `data_len` - The number of input bars.
/// * `options` - A slice containing the indicator options. `options[0]` is the period.
///
/// # Returns
///
/// `data_len - min_data(options) + 1` — the number of values in each output band.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the SMA Envelope indicator over the full input dataset.
///
/// The SMA Envelope plots three bands around a simple moving average:
/// - **lower**: `SMA - SMA * (percentage / 100)`
/// - **middle**: `SMA`
/// - **upper**: `SMA + SMA * (percentage / 100)`
///
/// # Inputs
///
/// * `inputs[0]` — real (price) values
///
/// # Options
///
/// * `options[0]` — period (SMA look-back window, must be ≥ 1)
/// * `options[1]` — percentage (envelope width as a percentage of the SMA, must be > 0)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `lower`, `outputs[1]` is `middle`,
/// `outputs[2]` is `upper`, and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming updates.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let percentage = options[1];

    let multipliers = multiplier(period, percentage);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut middle_band, mut upper_band, mut lower_band) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        )
    };

    let mut sum = init_state(real, period);
    cycle(
        real,
        period,
        multipliers,
        (&mut lower_band, &mut middle_band, &mut upper_band),
        &mut sum,
    );

    Ok((
        vec![lower_band, middle_band, upper_band],
        IndicatorState::new(real, sum, period, multipliers),
    ))
}

/// Performs the main calculation loop for the SMA Envelope indicator.
///
/// Iterates over `real[period..]`, advancing the rolling sum one bar at a time
/// and writing the lower, middle, and upper band values into `outputs`.
///
/// # Arguments
///
/// * `real` - A slice of real (price) values.
/// * `period` - The SMA look-back period.
/// * `multipliers` - Precomputed `(1/period, percentage/100)` tuple.
/// * `outputs` - A tuple of mutable slices `(lower, middle, upper)` to write into.
/// * `sum` - The running sum of the current SMA window (updated in place).
//#[inline(always)]
fn cycle(
    real: &[f64],
    period: usize,
    multipliers: (f64, f64),
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    sum: &mut f64,
) {
    let (lower_band, middle_band, upper_band) = outputs;

    for (j, i) in (period..real.len()).enumerate() {
        let (value, prev_value) =
            unsafe { (real.get_unchecked(i), real.get_unchecked(i - period)) };

        let (lower, middle, upper) = calc(sum, multipliers, value, prev_value);
        unsafe {
            *middle_band.get_unchecked_mut(j) = middle;
            *upper_band.get_unchecked_mut(j) = upper;
            *lower_band.get_unchecked_mut(j) = lower;
        }
    }
}

/// Calculates one output step of the SMA Envelope.
///
/// Updates the rolling `sum` by adding `value` and subtracting `prev_value`,
/// computes the SMA, then derives the envelope bands.
///
/// # Arguments
///
/// * `sum` - The running sum of the current SMA window (updated in place).
/// * `multipliers` - Precomputed `(1/period, percentage/100)` tuple.
/// * `value` - The current (newest) input price.
/// * `prev_value` - The price that is leaving the SMA window.
///
/// # Returns
///
/// `(lower, middle, upper)` where `middle` is the SMA, and `lower`/`upper`
/// are `middle ± middle * percentage`.
#[inline(always)]
pub fn calc(
    sum: &mut f64,
    multipliers: (f64, f64),
    value: &f64,
    prev_value: &f64,
) -> (f64, f64, f64) {
    let sma = calc_sma(sum, value, prev_value, &multipliers.0);
    let step = sma * multipliers.1;

    (sma - step, sma, sma + step)
}

pub fn multiplier(period: usize, percentage: f64) -> (f64, f64) {
    (sma_multiplier(period), percentage / 100.0)
}
