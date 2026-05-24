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
pub use crate::indicators::simd_indicators::linreg_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::linreg_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::linreg_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::linreg_simd::indicator_by_options as indicator;
}

/// Returns information about the Linear Regression (LINREG) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the LINREG indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "linreg",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Linear Regression",
        inputs: &["real"],
        options: &["period"],
        outputs: &["linreg"],
        optional_outputs: &["linregslope", "linregintercept"],
    }
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

        let (mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                slope_line: capacity,
                intercept_line: capacity
            );
            linreg_line = crate::uninit_vec!(f64, capacity);
        }

        cycle_linreg(
            &self.real,
            &mut self.state,
            self.period,
            &mut linreg_line,
            (&mut slope_line, &mut intercept_line),
        );
        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![linreg_line, slope_line, intercept_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub sum_x: f64,
    pub sum_y: f64,
    pub sum_xy: f64,
    pub per: f64,
}
impl State {
    pub fn new(sum_x: f64, sum_y: f64, sum_xy: f64, per: f64) -> Self {
        Self {
            sum_x,
            sum_y,
            sum_xy,
            per,
        }
    }

    pub fn init_state(data: &[f64], period: usize) -> Self {
        let (mut sum_x, mut sum_xx, mut sum_y, mut sum_xy) = (0.0, 0.0, 0.0, 0.0);
        if data.len() >= period - 1 {
            for i in 0..period - 1 {
                let d = unsafe { *data.get_unchecked(i) };
                sum_x += (i + 1) as f64;
                sum_xx += ((i + 1) as f64).powi(2);
                sum_y += d;
                sum_xy += (i + 1) as f64 * d;
            }
        }
        sum_x += period as f64;
        sum_xx += (period * period) as f64;
        let per = multiplier(period, sum_x, sum_xx);
        Self::new(sum_x, sum_y, sum_xy, per)
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
/// Returns the minimum amount of data required for the LINREG indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the LINREG calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the LINREG indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the LINREG calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Linear Regression (LINREG) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable optional outputs
///   (`linregslope`, `linregintercept`); `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `linreg`
/// - `outputs[1]` — `linregslope` (empty if not requested)
/// - `outputs[2]` — `linregintercept` (empty if not requested)
///
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
    let (mut linreg_line, mut slope_line, mut intercept_line);
    {
        let capacity = output_length(real.len(), options);
        (slope_line, intercept_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            slope_line: capacity,
            intercept_line: capacity
        );
        linreg_line = crate::uninit_vec!(f64, capacity);
    }
    let mut state = State::init_state(&real[1..period], period);
    // Perform the main LINREG calculation
    cycle_linreg(
        &real[1..],
        &mut state,
        period,
        &mut linreg_line,
        (&mut slope_line, &mut intercept_line),
    );

    Ok((
        vec![linreg_line, slope_line, intercept_line],
        IndicatorState::new(state, real, period),
    ))
}

/// Performs the main calculation loop for the LINREG indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the current `State`.
/// * `period` - The period for the LINREG calculation.
/// * `linreg_line` - A mutable slice for storing the LINREG output values.
/// * `out_vecs` - A tuple of mutable slices for optional slope and intercept outputs.
fn cycle_linreg(
    real: &[f64],
    state: &mut State,
    period: usize,
    linreg_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (slope_line, intercept_line) = out_vecs;
    let (has_optional, want_slope, want_intercept) =
        crate::calc_want_flags!(slope_line, intercept_line);

    for (j, i) in (period - 1..real.len()).enumerate() {
        let (prev_value, value) = unsafe { (*real.get_unchecked(j), *real.get_unchecked(i)) };
        let (linreg, slope, intercept) = calc(state, prev_value, value, period);

        unsafe { *linreg_line.get_unchecked_mut(j) = linreg };
        if has_optional {
            crate::store_optional_outputs!(j,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

#[inline(always)]
pub fn calc(state: &mut State, prev_value: f64, value: f64, period: usize) -> (f64, f64, f64) {
    let (sum_x, mut sum_y, mut sum_xy, per) = (state.sum_x, state.sum_y, state.sum_xy, state.per);
    let n = period as f64;

    sum_xy += value * n;
    sum_y += value;

    let slope = (n * sum_xy - sum_x * sum_y) * per;
    let intercept = (sum_y - slope * sum_x) / n;
    let linreg = intercept + slope * n;

    sum_xy -= sum_y;
    sum_y -= prev_value;

    (state.sum_y, state.sum_xy) = (sum_y, sum_xy);
    (linreg, slope, intercept)
}

/// Calculates the multiplier for the LINREG calculation.
#[inline]
pub fn multiplier(period: usize, sum_x: f64, sum_xx: f64) -> f64 {
    1.0 / (period as f64 * sum_xx - sum_x.powi(2))
}
