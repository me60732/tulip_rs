use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
//use std::slice::
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::min_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::min_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::min_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::min_simd::indicator_by_options as indicator;
}
use std::{
    f64,
    simd::{
        cmp::{SimdPartialEq, SimdPartialOrd},
        num::SimdFloat,
        Simd,
    },
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min: f64,
    pub trail: usize,
}

impl State {
    pub fn new(min: f64, trail: usize) -> Self {
        State { min, trail }
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

        let mut min_line = crate::uninit_vec!(f64, inputs[0].len());
        match self.periods.0 {
            1..=4 => {
                cycle_min::<1>(&self.real, self.periods, &mut min_line, &mut self.state);
            }
            5..24 => {
                cycle_min::<1>(&self.real, self.periods, &mut min_line, &mut self.state);
            }
            _ => {
                cycle_min::<1>(&self.real, self.periods, &mut min_line, &mut self.state);
            }
        }
        //cycle_min(&self.real, self.periods, &mut min_line, &mut self.state);

        self.real.drain(..self.real.len() - self.periods.1);

        Ok(vec![min_line])
    }
}
/// Returns information about the min indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the min indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "min",
        full_name: "minimum",
        display_type: DisplayType::Math,
        indicator_type: IndicatorType::Price,
        inputs: &["real"],
        options: &["period"],
        outputs: &["min"],
        optional_outputs: &[],
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
/// Returns the minimum amount of data required for the min indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the min calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}

/// Calculates the output length for the min indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the min calculation.
///
/// # Returns
///
/// The output length for the min calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Minimum Value (min) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `min`
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let periods = (options[0] as usize, options[0] as usize - 1);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut min_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::new(real[0], periods.0);
    match periods.0 {
        1..=4 => {
            cycle_min::<1>(real, periods, &mut min_line, &mut state);
        }
        5..30 => {
            cycle_min::<4>(real, periods, &mut min_line, &mut state);
        }
        _ => {
            cycle_min::<8>(real, periods, &mut min_line, &mut state);
        }
    }
    //cycle_min(real, periods, &mut min_line, &mut state);

    Ok((vec![min_line], IndicatorState::new(real, state, periods)))
}

/// Performs the main calculation loop for the min indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `periods` - A tuple of `(period, look_back)` for the min calculation.
/// * `min_line` - A mutable slice for storing the min output values.
/// * `state` - A mutable reference to the current `State`.
fn cycle_min<const N: usize>(
    real: &[f64],
    periods: (usize, usize),
    min_line: &mut [f64],
    state: &mut State,
) {
    for (j, i) in (periods.0 - 1..real.len()).enumerate() {
        unsafe {
            *min_line.get_unchecked_mut(j) = calc_unchecked::<N>(state, real, i, periods).0;
            //calc_unchecked::<N>(state, real, i, periods).0;
        }
    }
}
/// Calculates the minimum value in the window ending at index `i`.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current `State`.
/// * `real` - A slice of input data.
/// * `i` - The current index.
/// * `periods` - A tuple of `(period, look_back)` for the min calculation.
///
/// # Returns
///
/// A tuple containing the minimum value and the updated trail index.
///
/// ```
#[inline(always)]
pub fn calc(state: &mut State, real: &[f64], i: usize, periods: (usize, usize)) -> (f64, usize) {
    let (period, look_back) = periods;
    let (mut min, mut trail) = (state.min, state.trail);
    trail += 1;

    if period <= trail {
        let search_start = i - look_back;
        let search_end = i + 1;
        let window = &real[search_start..search_end];

        let (min_val, min_idx) = if period > 4 {
            find_min_simd::<4>(window)
        } else {
            find_min_scalar(window)
        };

        min = min_val;
        trail = i - (search_start + min_idx);
    } else {
        let current = real[i];
        if current <= min {
            min = current;
            trail = 0;
        }
    }

    state.min = min;
    state.trail = trail;
    (min, trail)
}
#[inline(always)]
pub unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    real: &[f64],
    i: usize,
    periods: (usize, usize),
) -> (f64, usize) {
    let (period, look_back) = periods;
    let (mut min, mut trail) = (state.min, state.trail);
    trail += 1;

    if period <= trail {
        let search_start = i - look_back;
        let search_end = i + 1;
        let window = real.get_unchecked(search_start..search_end);

        let (min_val, min_idx) = match N {
            1 => find_min_scalar(window),
            _ => find_min_simd::<N>(window),
        };

        min = min_val;
        trail = i - (search_start + min_idx);
    } else {
        let current = *real.get_unchecked(i);
        if current <= min {
            min = current;
            trail = 0;
        }
    }

    state.min = min;
    state.trail = trail;
    (min, trail)
}

#[inline(always)]
pub(crate) fn find_min_scalar(window: &[f64]) -> (f64, usize) {
    let end = window.len() - 1;
    let mut min_val = unsafe { *window.get_unchecked(end) };
    let mut min_idx = end;
    let mut i = end;

    while i > 0 {
        i -= 1;
        let val = unsafe { *window.get_unchecked(i) };
        if val < min_val {
            min_val = val;
            min_idx = i;
        }
    }

    (min_val, min_idx)
}

pub(crate) fn find_min_simd<const N: usize>(window: &[f64]) -> (f64, usize) {
    let mut global_min = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut min_idx = 0;

    let search_window = unsafe { window.get_unchecked(1..) };

    // Process chunks with SIMD - direct iteration
    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::from_slice(chunk);
        //let mask = values.simd_le(Simd::splat(global_min));
        let mask = values.simd_le(global_min);

        if mask.any() {
            global_min = Simd::splat(values.reduce_min());
            let eq_mask = values.simd_eq(global_min);

            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }

            min_idx = chunk_idx * N + i + 1;
        }
    }
    let mut global_min = global_min[0];
    // Handle remainder using find_min_scalar - calculate slice directly
    let processed_len = (search_window.len() / N) * N;
    let remainder = &search_window[processed_len..];
    if !remainder.is_empty() {
        let (rem_min, rem_idx) = find_min_scalar(remainder);
        if rem_min <= global_min {
            global_min = rem_min;
            min_idx = processed_len + 1 + rem_idx; // +1 for search_window offset
        }
    }

    (global_min, min_idx)
}
