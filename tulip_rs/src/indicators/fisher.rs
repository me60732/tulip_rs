use std::f64;

use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::State as MaxState;
use crate::indicators::medprice::calc as calc_medprice;
use crate::indicators::min::State as MinState;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::ring_buffer::single_buffer::mirror_buffer::{MinMaxBuffer, MirrorBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::fisher_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::fisher_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::fisher_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::fisher_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, period: usize) -> Self {
        Self { state, period }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut fisher_line, mut signal_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let [high, low] = inputs;

        match self.period {
            1..=12 => {
                cycle_fisher::<1>(
                    (high, low),
                    self.period,
                    (&mut fisher_line, &mut signal_line),
                    &mut self.state,
                );
            }
            13..30 => {
                cycle_fisher::<4>(
                    (high, low),
                    self.period,
                    (&mut fisher_line, &mut signal_line),
                    &mut self.state,
                );
            }
            _ => {
                cycle_fisher::<8>(
                    (high, low),
                    self.period,
                    (&mut fisher_line, &mut signal_line),
                    &mut self.state,
                );
            }
        }

        Ok(vec![fisher_line, signal_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub min_state: MinState,
    pub max_state: MaxState,
    pub val1: f64,
    pub fish: f64,
}

impl State {
    pub fn new(high: f64, low: f64, period: usize) -> Self {
        let medprice = calc_medprice(high, low);
        let mut buffer = Buffer::new(period);
        buffer.push(medprice);
        State {
            buffer,
            min_state: MinState::new(medprice, period),
            max_state: MaxState::new(medprice, period),
            val1: 0.0,
            fish: 0.0,
        }
    }

    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        fisher_line: &mut [f64],
        signal_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new(high[0], low[0], period);
        let mut i = 1;
        while state.buffer.get_count() < state.buffer.get_capacity() - 1 {
            state.buffer.push(calc_medprice(high[i], low[i]));
            i += 1;
        }
        (fisher_line[0], signal_line[0]) = calc::<1>(&mut state, high[i], low[i], period);
        state
    }
}

/// Returns information about the Fisher Transform indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Fisher Transform indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "fisher",
        full_name: "Fisher Transform",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["fisher", "fisher_signal"],
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

/// Returns the minimum amount of data required for the Fisher Transform indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Fisher Transform calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}

/// Returns the number of output values produced by the Fisher Transform indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Fisher Transform calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Fisher Transform indicator for an entire dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Outputs
///
/// * `outputs[0]` — `fisher` line
/// * `outputs[1]` — `fisher_signal` line
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; Fisher Transform has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `fisher` line,
/// `outputs[1]` is the `fisher_signal` line, and `state` can be passed
/// to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let period = options[0] as usize;
    let [high, low] = inputs;

    let (mut fisher_line, mut signal_line) = {
        let capacity = output_length(high.len(), options);
        (vec![0.0; capacity], vec![0.0; capacity])
    };

    let mut state = State::init_state(high, low, period, &mut fisher_line, &mut signal_line);

    let outputs = (&mut fisher_line[1..], &mut signal_line[1..]);
    let inputs = (&high[period..], &low[period..]);
    match period {
        1..=4 => {
            cycle_fisher::<1>(inputs, period, outputs, &mut state);
        }
        5..30 => {
            cycle_fisher::<4>(inputs, period, outputs, &mut state);
        }
        _ => {
            cycle_fisher::<8>(inputs, period, outputs, &mut state);
        }
    }
    Ok((
        vec![fisher_line, signal_line],
        IndicatorState { state, period },
    ))
}

/// Performs the main calculation loop for the Fisher Transform indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple containing high and low price slices.
/// * `period` - The period for the Fisher Transform calculation.
/// * `output_lines` - A tuple containing mutable references to fisher and signal vectors.
/// * `state` - A mutable reference to the indicator state.
fn cycle_fisher<const N: usize>(
    inputs: (&[f64], &[f64]),
    period: usize,
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State,
) {
    let (fisher_line, signal_line) = output_lines;
    let (high, low) = inputs;
    for i in 0..high.len() {
        let (h, l) = unsafe { (*high.get_unchecked(i), *low.get_unchecked(i)) };
        let (fisher, signal) = calc::<N>(state, h, l, period);
        unsafe {
            *fisher_line.get_unchecked_mut(i) = fisher;
            *signal_line.get_unchecked_mut(i) = signal;
        }
    }
}

#[inline(always)]
pub fn calc<const N: usize>(state: &mut State, high: f64, low: f64, period: usize) -> (f64, f64) {
    let medprice = calc_medprice(high, low);

    //unsafe { state.buffer.push_unchecked(medprice); }
    state.buffer.push(medprice);
    let (min, _) = state
        .buffer
        .min::<N>(&mut state.min_state, medprice, period);
    let (max, _) = state
        .buffer
        .max::<N>(&mut state.max_state, medprice, period);

    calc_fisher(min, max, medprice, state)
}

#[inline(always)]
fn calc_fisher(min: f64, max: f64, medprice: f64, state: &mut State) -> (f64, f64) {
    // Correctly named constants
    const PRICE_WEIGHT: f64 = 0.66; // 0.33 * 2.0 - weight for new normalized price
    const SMOOTH_WEIGHT: f64 = 0.67; // smoothing factor for exponential average
    const MIN_MM: f64 = 0.001;

    let mut val1 = state.val1;
    let mm = (max - min).max(MIN_MM);

    // Use mul_add for better precision
    val1 = PRICE_WEIGHT.mul_add((medprice - min) / mm - 0.5, SMOOTH_WEIGHT * val1);

    // Clamp val1 to the range [-0.999, 0.999]
    if val1 > 0.99 {
        val1 = 0.999;
    } else if val1 < -0.99 {
        val1 = -0.999;
    }
    state.val1 = val1;

    let signal = state.fish;

    let ln_arg = (1.0 + val1) / (1.0 - val1);

    state.fish = 0.5 * (ln_arg.ln() + signal); //state.fish);
    (state.fish, signal)
}
