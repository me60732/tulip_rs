use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::linreg::State as LinregState;
use crate::indicators::tsf::{calc as calc_tsf, output_length as tsf_output_length};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_options as indicator;
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
        inputs: &[&[f64]; 1],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut fosc_line, mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (tsf_line, linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                tsf_line: capacity,
                linreg_line: capacity,
                slope_line: capacity,
                intercept_line: capacity
            );
            fosc_line = crate::uninit_vec!(f64, capacity);
        }
        //let mut fosc_line = Vec::<f64>::with_capacity(capacity); //vec![0.0; capacity];
        // Perform the main FOSC calculation
        cycle_fosc(
            &self.real,
            &mut self.state,
            self.period,
            self.period - 1,
            (
                &mut fosc_line,
                &mut tsf_line,
                &mut linreg_line,
                &mut slope_line,
                &mut intercept_line,
            ),
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![
            fosc_line,
            tsf_line,
            linreg_line,
            slope_line,
            intercept_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub linreg_state: LinregState,
    pub tsf: f64,
}
impl State {
    pub fn new(tsf: f64, sum_x: f64, sum_y: f64, sum_xy: f64, per: f64) -> Self {
        Self {
            tsf,
            linreg_state: LinregState::new(sum_x, sum_y, sum_xy, per),
        }
    }
    pub fn init_state(
        real: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> Self {
        let (tsf_line, linreg_line, slope_line, intercept_line) = out_vecs;
        let (has_optional, _, _, _, _) =
            crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);
        let mut state = Self {
            tsf: 0.0,
            linreg_state: LinregState::init_state(&real[1..period], period),
        };
        let (_, tsf, linreg, slope, intercept) = calc(&mut state, real[1], real[period], period);
        if has_optional {
            crate::init_store_optional_outputs!(period, real.len(),
                tsf_line => tsf,
                linreg_line => linreg,
                slope_line => slope,
                intercept_line => intercept
            );
        }
        state
    }
}

/// Returns information about the Forecast Oscillator (FOSC) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the FOSC indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "fosc",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        full_name: "Forecast Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["fosc"],
        optional_outputs: &["tsf", "linreg", "linregslope", "linregintercept"],
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
/// Returns the minimum amount of data required for the FOSC indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the FOSC calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 2
}

/// Returns the number of output values produced by the FOSC indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the FOSC calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Forecast Oscillator (FOSC) indicator for an entire dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Outputs
///
/// * `outputs[0]` — `fosc` line
/// * `outputs[1]` — `tsf` (optional, if requested)
/// * `outputs[2]` — `linreg` (optional, if requested)
/// * `outputs[3]` — `linregslope` (optional, if requested)
/// * `outputs[4]` — `linregintercept` (optional, if requested)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice selecting which extra outputs to compute:
///   index `0` = `tsf`, `1` = `linreg`, `2` = `linregslope`, `3` = `linregintercept`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `fosc` line and
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
    let (mut fosc_line, mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
    {
        let capacity = output_length(real.len(), options);
        let tsf_capacity = tsf_output_length(real.len(), options);
        (tsf_line, linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false, false],
            tsf_line: tsf_capacity,
            linreg_line: tsf_capacity,
            slope_line: tsf_capacity,
            intercept_line: tsf_capacity
        );

        fosc_line = crate::uninit_vec!(f64, capacity);
    }
    let mut state = State::init_state(
        real,
        period,
        (
            &mut tsf_line,
            &mut linreg_line,
            &mut slope_line,
            &mut intercept_line,
        ),
    );
    let outputs = {
        let offsets = crate::slice_outputs_start!(
            fosc_line.len(),
            tsf_line,
            linreg_line,
            slope_line,
            intercept_line
        );
        (
            fosc_line.as_mut_slice(),
            &mut tsf_line[offsets.0..],
            &mut linreg_line[offsets.1..],
            &mut slope_line[offsets.2..],
            &mut intercept_line[offsets.3..],
        )
    };

    // Perform the main FOSC calculation
    cycle_fosc(&real[2..], &mut state, period, period - 1, outputs);

    Ok((
        vec![fosc_line, tsf_line, linreg_line, slope_line, intercept_line],
        IndicatorState::new(state, real, period),
    ))
}

/// Performs the main calculation loop for the FOSC indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the indicator state.
/// * `period` - The period for the FOSC calculation.
/// * `start` - The starting index within `real` for the calculation.
/// * `out_vecs` - A tuple of mutable output slices:
///   `(fosc_line, tsf_line, linreg_line, slope_line, intercept_line)`.
//#[inline(always)]
fn cycle_fosc(
    real: &[f64],
    state: &mut State,
    period: usize,
    start: usize,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (fosc_line, tsf_line, linreg_line, slope_line, intercept_line) = out_vecs;
    let (has_optional, want_tsf, want_linreg, want_slope, want_intercept) =
        crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);

    //for (i, &value) in real.iter().enumerate().skip(start) {
    for (j, i) in (start..real.len()).enumerate() {
        let prev_value = unsafe { *real.get_unchecked(j) };
        let value = unsafe { *real.get_unchecked(i) };
        let (fosc, tsf, linreg, slope, intercept) = calc(state, prev_value, value, period);

        unsafe { *fosc_line.get_unchecked_mut(j) = fosc };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_tsf, tsf_line => tsf,
                want_linreg, linreg_line => linreg,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

#[inline(always)]
pub fn calc(
    state: &mut State,
    prev_value: f64,
    value: f64,
    period: usize,
) -> (f64, f64, f64, f64, f64) {
    let fosc = 100.0 * (value - state.tsf) / value; //.max(f64::EPSILON);

    let (tsf, linreg, slope, intercept) =
        calc_tsf(&mut state.linreg_state, prev_value, value, period);
    state.tsf = tsf;
    (fosc, tsf, linreg, slope, intercept)
}
