use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::linreg::calc as calc_linreg;
pub use crate::indicators::linreg::State;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_options as indicator;
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    real: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, real: &[f64], period: usize) -> Self {
        Self {
            state,
            real: real[real.len() - period + 1..].to_vec(),
            period,
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

        let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                linreg_line: capacity,
                slope_line: capacity,
                intercept_line: capacity
            );
            tsf_line = crate::uninit_vec!(f64, capacity);
        }
        cycle_tsf(
            &self.real,
            &mut self.state,
            self.period,
            &mut tsf_line,
            (&mut linreg_line, &mut slope_line, &mut intercept_line),
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![tsf_line, linreg_line, slope_line, intercept_line])
    }
}
/// Returns information about the Time Series Forecast (TSF) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TSF indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "tsf",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Time Series Forecast",
        inputs: &["real"],
        options: &["period"],
        outputs: &["tsf"],
        optional_outputs: &["linreg", "linregslope", "linregintercept"],
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
/// Returns the minimum amount of data required for the TSF indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the TSF calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TSF calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Time Series Forecast (TSF) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (price series)
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice controlling extra output series;
///   `optional_outputs[0] = true` enables `linreg`, `optional_outputs[1] = true` enables
///   `linregslope`, `optional_outputs[2] = true` enables `linregintercept`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `tsf`,
/// `outputs[1]` is `linreg` (empty unless requested),
/// `outputs[2]` is `linregslope` (empty unless requested),
/// `outputs[3]` is `linregintercept` (empty unless requested), and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];
    let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
    {
        let capacity = output_length(real.len(), options);
        (linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            linreg_line: capacity,
            slope_line: capacity,
            intercept_line: capacity
        );
        tsf_line = crate::uninit_vec!(f64, capacity); //Vec::with_capacity(capacity);
    }
    let mut state = State::init_state(&real[1..period], period);

    // Perform the main TSF calculation
    cycle_tsf(
        &real[1..],
        &mut state,
        period,
        &mut tsf_line,
        (&mut linreg_line, &mut slope_line, &mut intercept_line),
    );

    Ok((
        vec![tsf_line, linreg_line, slope_line, intercept_line],
        IndicatorState::new(state, real, period),
    ))
}

/// Performs the main calculation loop for the TSF indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the current linear regression state.
/// * `period` - The period for the TSF calculation.
/// * `tsf_line` - A mutable slice for storing the TSF output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(linreg_line, slope_line, intercept_line)`.
fn cycle_tsf(
    real: &[f64],
    state: &mut State,
    period: usize,
    tsf_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (linreg_line, slope_line, intercept_line) = out_vecs;
    let (has_optional, want_linreg, want_slope, want_intercept) =
        crate::calc_want_flags!(linreg_line, slope_line, intercept_line);

    for (j, i) in (period - 1..real.len()).enumerate() {
        let (prev_value, value) = unsafe { (*real.get_unchecked(j), *real.get_unchecked(i)) };
        let (tsf, linreg, slope, intercept) = calc(state, prev_value, value, period);

        unsafe { *tsf_line.get_unchecked_mut(j) = tsf };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_linreg, linreg_line => linreg,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

/// Calculates the Time Series Forecast (TSF) for the current data point.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current linear regression state.
/// * `prev_value` - The oldest value leaving the rolling window.
/// * `value` - The newest value entering the rolling window.
/// * `period` - The period for the TSF calculation.
///
/// # Returns
///
/// A tuple `(tsf, linreg, slope, intercept)` containing the forecast value, linear regression
/// value, slope, and intercept for the current data point.
#[inline(always)]
pub fn calc(state: &mut State, prev_value: f64, value: f64, period: usize) -> (f64, f64, f64, f64) {
    let (linreg, slope, intercept);
    (linreg, slope, intercept) = calc_linreg(state, prev_value, value, period);
    //let tsf = intercept + slope * (period + 1) as f64;
    let tsf = slope.mul_add((period + 1) as f64, intercept);
    (tsf, linreg, slope, intercept)
}
