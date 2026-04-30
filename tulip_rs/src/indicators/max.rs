use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::max_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::max_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::max_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::max_simd::indicator_by_options as indicator;
}

use std::simd::{cmp::SimdPartialEq, cmp::SimdPartialOrd, num::SimdFloat, Simd};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub max: f64,
    pub trail: usize,
}

impl State {
    pub fn new(max: f64, trail: usize) -> Self {
        State { max, trail }
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub real: Vec<f64>,
    pub state: State,
    pub periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, periods: (usize, usize)) -> Self {
        Self {
            real: real[real.len() - periods.1..].to_vec(),
            state,
            periods,
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

        let mut max_line = crate::uninit_vec!(f64, inputs[0].len());

        match self.periods.0 {
            1..=4 => {
                cycle_max::<1>(&self.real, self.periods, &mut max_line, &mut self.state);
            }
            5..30 => {
                cycle_max::<4>(&self.real, self.periods, &mut max_line, &mut self.state);
            }
            _ => {
                cycle_max::<8>(&self.real, self.periods, &mut max_line, &mut self.state);
            }
        }

        self.real.drain(..self.real.len() - self.periods.1);

        Ok(vec![max_line])
    }
}
/// Returns information about the max indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the max indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "max",
        full_name: "maximum",
        display_type: DisplayType::Math,
        indicator_type: IndicatorType::Price,
        inputs: &["real"],
        options: &["period"],
        outputs: &["max"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the maximum amount of data required for the max indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the max calculation.
///
/// # Returns
///
/// The maximum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the max calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length for the max calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the max indicator values.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (real prices).
/// * `options` - A slice containing the options for the max calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the max indicator values and the state.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let periods = (options[0] as usize, options[0] as usize - 1);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut max_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::new(real[0], periods.0);
    //let mut state = init_state(real, period);
    match periods.0 {
        1..=4 => {
            cycle_max::<1>(real, periods, &mut max_line, &mut state);
        }
        5..30 => {
            cycle_max::<4>(real, periods, &mut max_line, &mut state);
        }
        _ => {
            cycle_max::<8>(real, periods, &mut max_line, &mut state);
        }
    }

    Ok((
        vec![max_line],
        IndicatorState::new(real, state, periods)
    ))
}

/// Performs the main calculation loop for the max indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the max calculation.
/// * `max_line` - A mutable reference to a vector for storing the max line.
/// * `maxi` - The index of the maximum value.
/// * `start` - The starting index for the calculation.
fn cycle_max<const N: usize>(
    real: &[f64],
    periods: (usize, usize),
    max_line: &mut [f64],
    state: &mut State,
) {
    for (j, i) in (periods.1..real.len()).enumerate() {
        unsafe {
            *max_line.get_unchecked_mut(j) = calc_unchecked::<N>(state, real, i, periods).0;
        }
    }
}
/// Performs the main calculation loop for the max indicator.
/// Calculates the maximum value in the given period.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the max calculation.
/// * `i` - The current index.
/// * `maxi` - The index of the maximum value.
/// * `value` - The current value.
///
/// # Returns
///
/// A tuple containing the maximum value and the updated index of the maximum value.
///
/// ```
#[inline(always)]
pub fn calc(state: &mut State, real: &[f64], i: usize, periods: (usize, usize)) -> (f64, usize) {
    let (period, look_back) = periods;
    let (mut max, mut trail) = (state.max, state.trail);
    trail += 1;

    if period <= trail {
        let search_start = i - look_back;
        let search_end = i + 1;
        let window = &real[search_start..search_end];

        let (max_val, max_idx) = if period > 13 {
            find_max_simd::<4>(window)
        } else {
            find_max_scalar(window)
        };

        max = max_val;
        trail = i - (search_start + max_idx);
    } else {
        let current = real[i];
        if current >= max {
            // >= to handle equal values correctly
            max = current;
            trail = 0;
        }
    }

    state.max = max;
    state.trail = trail;
    (max, trail)
}

#[inline(always)]
pub unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    real: &[f64],
    i: usize,
    periods: (usize, usize),
) -> (f64, usize) {
    let (period, look_back) = periods;
    let (mut max, mut trail) = (state.max, state.trail);
    trail += 1;

    if period <= trail {
        let search_start = i - look_back;
        let search_end = i + 1;
        let window = real.get_unchecked(search_start..search_end);

        let (max_val, max_idx) = match N {
            1 => find_max_scalar(window),
            _ => find_max_simd::<N>(window),
        };

        max = max_val;
        trail = i - (search_start + max_idx);
    } else {
        let current = *real.get_unchecked(i);
        if current >= max {
            max = current;
            trail = 0;
        }
    }

    state.max = max;
    state.trail = trail;
    (max, trail)
}
#[inline(always)]
pub(crate) fn find_max_scalar(window: &[f64]) -> (f64, usize) {
    let mut max_val = window[0];
    let mut max_idx = 0;

    for i in 1..window.len() {
        if window[i] >= max_val {
            // >= to get last position
            max_val = window[i];
            max_idx = i;
        }
    }
    (max_val, max_idx)
}

pub(crate) fn find_max_simd<const N: usize>(window: &[f64]) -> (f64, usize) {
    let mut global_max = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut max_idx = 0;

    let search_window = unsafe { window.get_unchecked(1..) };

    // Process chunks with SIMD - direct iteration
    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::from_slice(chunk);
        //let mask = values.simd_le(Simd::splat(global_max));
        let mask = values.simd_ge(global_max);

        if mask.any() {
            global_max = Simd::splat(values.reduce_max());
            let eq_mask = values.simd_eq(global_max);

            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }

            max_idx = chunk_idx * N + i + 1;
        }
    }
    let mut global_max = global_max[0];
    // Handle remainder using find_max_scalar - calculate slice directly
    let processed_len = (search_window.len() / N) * N;
    let remainder = &search_window[processed_len..];
    if !remainder.is_empty() {
        let (rem_max, rem_idx) = find_max_scalar(remainder);
        if rem_max >= global_max {
            global_max = rem_max;
            max_idx = processed_len + 1 + rem_idx; // +1 for search_window offset
        }
    }

    (global_max, max_idx)
}
