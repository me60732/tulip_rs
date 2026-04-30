use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::aroon::State;
use crate::indicators::aroon::{calc as calc_aroon, calc_unchecked as calc_unchecked_aroon};
pub use crate::indicators::aroon::{multiplier, OPTIONS_WIDTH};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
    period: usize,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(high: &[f64], low: &[f64], state: State, period: usize, multiplier: f64) -> Self {
        Self {
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            state,
            period,
            multiplier,
        }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let period = self.period;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let capacity = inputs[0].len();
        let mut aroonosc_line = crate::uninit_vec!(f64, capacity);

        let (mut aroon_up_line, mut aroon_down_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            aroon_up_line: capacity,
            aroon_down_line: capacity
        );
        match period {
            1..=4 => {
                cycle::<1>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    &mut aroonosc_line,
                    &mut self.state,
                    (&mut aroon_down_line, &mut aroon_up_line),
                );
            }
            5..30 => {
                cycle::<4>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    &mut aroonosc_line,
                    &mut self.state,
                    (&mut aroon_down_line, &mut aroon_up_line),
                );
            }
            _ => {
                cycle::<8>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    &mut aroonosc_line,
                    &mut self.state,
                    (&mut aroon_down_line, &mut aroon_up_line),
                );
            }
        }

        self.high.drain(..self.high.len() - period);
        self.low.drain(..self.low.len() - period);

        Ok(vec![aroonosc_line, aroon_down_line, aroon_up_line])
    }
}
/// Returns information about the Aroon Oscillator indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Aroon Oscillator indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "aroonosc",
        full_name: "Aroon Oscillator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["aroonosc"],
        optional_outputs: &["aroon_down", "aroon_up"],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Aroon Oscillator indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Aroon Oscillator calculation.
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
/// * `options` - A slice containing the options for the Aroon Oscillator calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length for the Aroon Oscillator calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Aroon Oscillator indicator values.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (high and low prices).
/// * `options` - A slice containing the options for the Aroon Oscillator calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the Aroon Oscillator indicator values and the state.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let period = options[0] as usize;
    let multiplier = multiplier(period);
    let high = inputs[0];
    let low = inputs[1];

    let capacity = output_length(high.len(), options);
    //let mut aroonosc_line = vec![0.0; capacity]; //Vec::with_capacity(capacity);
    let mut aroonosc_line = crate::uninit_vec!(f64, capacity);

    let (mut aroon_up_line, mut aroon_down_line) = crate::init_optional_outputs_eff!(
        optional_outputs, &[false, false],
        aroon_up_line: capacity,
        aroon_down_line: capacity
    );

    let mut state = State::init_state(high, low, period);
    match period {
        1..=4 => {
            cycle::<1>(
                (&high, &low),
                period,
                multiplier,
                &mut aroonosc_line,
                &mut state,
                (&mut aroon_down_line, &mut aroon_up_line),
            );
        }
        5..30 => {
            cycle::<4>(
                (&high, &low),
                period,
                multiplier,
                &mut aroonosc_line,
                &mut state,
                (&mut aroon_down_line, &mut aroon_up_line),
            );
        }
        _ => {
            cycle::<8>(
                (&high, &low),
                period,
                multiplier,
                &mut aroonosc_line,
                &mut state,
                (&mut aroon_down_line, &mut aroon_up_line),
            );
        }
    }

    Ok((
        vec![aroonosc_line, aroon_down_line, aroon_up_line],
        IndicatorState {
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            state,
            period,
            multiplier,
        },
    ))
}

/// Performs the main calculation loop for the Aroon Oscillator indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `period` - The period for the Aroon Oscillator calculation.
/// * `aroonosc` - A mutable reference to a vector for storing the Aroon Oscillator line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
/// * `min` - The minimum value.
/// * `max` - The maximum value.
/// * `trail_min` - The trailing index for the minimum value.
/// * `trail_max` - The trailing index for the maximum value.
///
/// # Returns
///
/// A tuple containing the updated min, max, trail_min, and trail_max values.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64]),
    period: usize,
    multiplier: f64,
    aroonosc_line: &mut [f64],
    state: &mut State,
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let high = inputs.0;

    let (aroon_down_line, aroon_up_line) = out_vecs;
    let (has_optional, want_up, want_down) =
        crate::calc_want_flags!(aroon_up_line, aroon_down_line);

    for (j, i) in (period..high.len()).enumerate() {
        let (aroonosc, aroon_down, aroon_up) =
            unsafe { calc_unchecked::<N>(state, inputs, i, period, multiplier) };
        unsafe { *aroonosc_line.get_unchecked_mut(j) = aroonosc };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_up, aroon_up_line => aroon_up,
                want_down, aroon_down_line => aroon_down
            );
        }
    }
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64]),
    i: usize,
    period: usize,
    multiplier: f64,
) -> (f64, f64, f64) {
    let (aroon_down, aroon_up) = calc_aroon(state, inputs, i, period, multiplier);

    (aroon_up - aroon_down, aroon_down, aroon_up)
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    inputs: (&[f64], &[f64]),
    i: usize,
    period: usize,
    multiplier: f64,
) -> (f64, f64, f64) {
    let (aroon_down, aroon_up) = calc_unchecked_aroon::<N>(state, inputs, i, period, multiplier);

    (aroon_up - aroon_down, aroon_down, aroon_up)
}
