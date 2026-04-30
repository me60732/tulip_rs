use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::sma::calc as calc_sma;
pub use crate::indicators::sma::multiplier;
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::stddev_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::stddev_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::stddev_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::stddev_simd::indicator_by_options as indicator;
}

use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State,
    period: usize,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, multiplier: f64, period: usize) -> Self {
        let real = real[real.len() - period..].to_vec();
        Self {
            real,
            state,
            period,
            multiplier,
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

        let (mut stddev_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_stddev(
            &self.real,
            &mut self.state,
            self.period,
            self.multiplier,
            &mut stddev_line,
            &mut sma_line,
        );

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![stddev_line, sma_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub sum: f64,
    pub sum_sq: f64,
}
impl State {
    pub fn new(sum: f64, sum_sq: f64) -> Self {
        State { sum, sum_sq }
    }

    pub fn init_state(real: &[f64], period: usize) -> State {
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for i in 0..period {
            sum += real[i];
            sum_sq = real[i].mul_add(real[i], sum_sq);
        }
        State::new(
            sum,
            sum_sq, /*real[0..period].iter().sum::<f64>(),
                   real[0..period].iter().map(|&x| x * x).sum::<f64>(),*/
        )
    }
    #[inline(always)]
    pub fn calc(&mut self, value: &f64, prev_value: &f64, multiplier: f64) -> (f64, f64) {
        let sma = calc_sma(&mut self.sum, value, prev_value, &multiplier);
        self.sum_sq += value.mul_add(*value, -(prev_value * prev_value));
        let mut sd = self.sum_sq.mul_add(multiplier, -(sma * sma));
        sd = sd.sqrt().max(f64::EPSILON);

        (sd, sma)
    }
}

/// Returns information about the Standard Deviation (STDDEV) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the STDDEV indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "stddev",
        display_type: DisplayType::Math,
        indicator_type: IndicatorType::Volatility,
        full_name: "Standard Deviation",
        inputs: &["real"],
        options: &["period"],
        outputs: &["stddev"],
        optional_outputs: &["sma"],
    }
}
pub fn min_data_accuracy(options: &[f64; OPTIONS_WIDTH], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the STDDEV indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the STDDEV calculation.
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
/// * `options` - A slice containing the options for the STDDEV calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64; OPTIONS_WIDTH]) -> usize {
    data_len - min_data(options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut stddev_line, mut sma_line) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                sma_line: capacity
            ),
        )
    };

    let mut state = State::init_state(real, period);

    cycle_stddev(
        &real,
        &mut state,
        period,
        multiplier,
        &mut stddev_line,
        &mut sma_line,
    );

    Ok((
        vec![stddev_line, sma_line],
        IndicatorState {
            period,
            multiplier,
            state,
            real: real[real.len() - period..].to_vec(),
        },
    ))
}

/// Performs the main calculation loop for the STDDEV indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `period` - The period for the STDDEV calculation.
/// * `stddev_line` - A mutable reference to a vector for storing the STDDEV line.
/// * `output_vectors` - A mutable reference to a slice of optional output vectors.
fn cycle_stddev(
    real: &[f64],
    state: &mut State,
    period: usize,
    multiplier: f64,
    stddev_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (stddev, sma) = unsafe {
            state.calc(
                real.get_unchecked(i),
                real.get_unchecked(j),
                multiplier,
            )
        };
        unsafe { *stddev_line.get_unchecked_mut(j) = stddev };
        crate::store_optional_outputs!(j,
            want_sma, sma_line => sma
        );
    }
}

/// Calculates the current Standard Deviation (STDDEV) value.
///
/// # Arguments
///
/// * `value` - The current input value.
/// * `prev_value` - The previous input value.
/// * `sum` - The sum of the previous input values.
/// * `sum_sq` - The sum of the squares of the previous input values.
/// * `period` - The period for the STDDEV calculation.
///
/// # Returns
///
/// The STDDEV value, the updated sum, and the SMA.
#[inline(always)]
pub fn calc(state: &mut State, value: &f64, prev_value: &f64, multiplier: f64) -> (f64, f64) {
    state.calc(value, prev_value, multiplier)
}
