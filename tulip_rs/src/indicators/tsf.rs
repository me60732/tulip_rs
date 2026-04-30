use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::linreg::calc as calc_linreg;
pub use crate::indicators::linreg::State;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::tsf_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
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
            real: real[real.len() - period+1..].to_vec(),
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

        self.real.drain(..self.real.len() - self.period+1);

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

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TSF calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Time Series Forecast (TSF) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the TSF calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// A vector of vectors containing the TSF line.

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

/// Calculates the Time Series Forecast (TSF) from a previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the TSF calculation.
/// * `prev_state` - A reference to the previous state containing the previous input values.
///
/// # Returns
///
/// A vector of vectors containing the TSF line.

/// Performs the main calculation loop for the TSF indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the TSF calculation.
/// * `start` - The starting index for the calculation.
/// * `tsf_line` - A mutable reference to a vector for storing the TSF line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
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
    
    for (j, i) in (period-1..real.len()).enumerate() {
        let (prev_value, value) = unsafe { (
            *real.get_unchecked(j),
            *real.get_unchecked(i)
        ) };
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

/// Calculates the Time Series Forecast (TSF) for the current data point using rolling sums.
///
/// # Arguments
///
/// * `sum_x` - The sum of x values.
/// * `sum_y` - A mutable reference to the sum of y values.
/// * `sum_xy` - A mutable reference to the sum of x * y values.
/// * `prev_value` - The previous y value.
/// * `value` - The new y value.
/// * `period` - The period for the TSF calculation.
/// * `per` - The precomputed multiplier.
///
/// # Returns
///
/// The calculated TSF value, slope, and intercept.
#[inline(always)]
pub fn calc(state: &mut State, prev_value: f64, value: f64, period: usize) -> (f64, f64, f64, f64) {
    let (linreg, slope, intercept);
    (linreg, slope, intercept) = calc_linreg(state, prev_value, value, period);
    //let tsf = intercept + slope * (period + 1) as f64;
    let tsf = slope.mul_add((period + 1) as f64, intercept);
    (tsf, linreg, slope, intercept)
}
