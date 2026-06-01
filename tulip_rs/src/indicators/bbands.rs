use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::stddev::calc as stddev_calc;
pub use crate::indicators::stddev::{multiplier, State};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options as indicator;
}

/// Returns information about the Bollinger Bands (BBANDS) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the BBANDS indicator.
pub const INFO: Info = Info {
    name: "bbands",
    full_name: "Bollinger Bands",
    indicator_type: IndicatorType::Volatility,
    inputs: &["real"],
    options: &["period", "std_dev"],
    outputs: &["bbands_lower", "bbands_middle", "bbands_upper"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "bbands",
        label: "BBANDS",
        display_type: DisplayType::Overlay,
        outputs: &["bbands_lower", "bbands_middle", "bbands_upper"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State,
    period: usize,
    multiplier: f64,
    std_dev: f64,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, period: usize, multiplier: f64, std_dev: f64) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
            multiplier,
            std_dev,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        let period = self.period;

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

        cycle_bbands(
            &self.real,
            period,
            self.std_dev,
            self.multiplier,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut self.state,
        );

        self.real.drain(..self.real.len() - period);

        Ok(vec![lower_band, middle_band, upper_band])
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
/// Returns the minimum amount of data required for the BBANDS indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the BBANDS calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the BBANDS indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the BBANDS calculation.
///
/// # Returns
///
/// The number of output values produced by the BBANDS calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Bollinger Bands (BBANDS) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (price) values
///
/// # Options
///
/// * `options[0]` — period
/// * `options[1]` — std_dev (standard deviation multiplier for the bands)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; BBANDS has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `bbands_lower`, `outputs[1]` is `bbands_middle`,
/// `outputs[2]` is `bbands_upper`, and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let std_dev = options[1];

    let multiplier = multiplier(period);

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

    let mut state = State::new(
        real[0..period].iter().sum::<f64>(),
        real[0..period].iter().map(|&x| x * x).sum::<f64>(),
    );
    cycle_bbands(
        real,
        period,
        std_dev,
        multiplier,
        (&mut lower_band, &mut middle_band, &mut upper_band),
        &mut state,
    );

    Ok((
        vec![lower_band, middle_band, upper_band],
        IndicatorState {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
            multiplier,
            std_dev,
        },
    ))
}

/// Performs the main calculation loop for the BBANDS indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `period` - The period for the BBANDS calculation.
/// * `std_dev` - The standard deviation multiplier for the bands.
/// * `multiplier` - The precomputed period multiplier used in standard deviation calculation.
/// * `outputs` - A tuple of mutable slices for storing the lower, middle, and upper bands.
/// * `state` - A mutable reference to the current indicator state.
#[inline(always)]
fn cycle_bbands(
    real: &[f64],
    period: usize,
    std_dev: f64,
    multiplier: f64,
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State,
) {
    let (lower_band, middle_band, upper_band) = outputs;

    for (j, i) in (period..real.len()).enumerate() {
        let prev_value = unsafe { real.get_unchecked(i - period) };
        //let prev_value = &real[i - period];
        let (lower, middle, upper) = calc(state, &std_dev, multiplier, unsafe { real.get_unchecked(i) }, prev_value);
        unsafe {
            *middle_band.get_unchecked_mut(j) = middle;
            *upper_band.get_unchecked_mut(j) = upper;
            *lower_band.get_unchecked_mut(j) = lower;
        }
    }
}

/// Calculates the current Bollinger Bands values.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current standard deviation state.
/// * `std_dev` - The standard deviation multiplier for the bands.
/// * `multiplier` - The precomputed period multiplier used in standard deviation calculation.
/// * `value` - The current input value.
/// * `prev_value` - The previous period's input value (used to update the rolling sum).
///
/// # Returns
///
/// A tuple of `(lower_band, middle_band, upper_band)`.
#[inline(always)]
pub fn calc(
    state: &mut State,
    std_dev: &f64,
    multiplier: f64,
    value: &f64,
    prev_value: &f64,
) -> (f64, f64, f64) {
    let (sd, sma);
    (sd, sma) = stddev_calc(state, value, prev_value, multiplier);

    let upper_band = std_dev.mul_add(sd, sma);
    //let upper_band = sma + sd * std_dev;
    let lower_band = (-std_dev).mul_add(sd, sma);
    //let lower_band = sma - sd * std_dev;
    (lower_band, sma, upper_band)
}
