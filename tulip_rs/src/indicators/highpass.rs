//! # Ehlers High Pass Filter
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 2.
//! Also described in *Cybernetic Analysis for Stocks and Futures* (2004).
//!
//! A one-pole IIR high-pass filter that removes the trend (DC component and
//! sub-cycle drift) from price, leaving only the oscillatory cycle content for
//! downstream analysis. The single `period` option sets the cutoff: cycles
//! longer than `period` bars are suppressed; shorter cycles pass through.
//!
//! ## Formula
//!
//! Given `ω = 2π / period`:
//!
//! ```text
//! α  = (1 − sin ω) / cos ω
//! b  = (1 + α) / 2
//! HP = b·(Price − Price[1]) + α·HP[1]
//! ```
//!
//! Internally the coefficients are stored as `(a1, a2) = (α, b)` and evaluated as
//! `HP = a1·HP[1] + a2·(Price − Price[1])`.
//!
//! ## Role in this library
//!
//! Used as the first stage of the [`roofingfilter`] and (transitively) the
//! [`hilberttransform`] indicator. On its own it outputs the de-trended signal,
//! which is not directly tradeable but is essential for cycle-period estimation.


use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_options as indicator;
}

/// Returns metadata for the Ehlers Super Smoother indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the SuperSmoother indicator, including
/// its input (`real`), configurable `period`, and output line (`supersmoother`).
pub const INFO: Info = Info {
    name: "highpass",
    indicator_type: IndicatorType::Trend,
    full_name: "Ehlers High Pass Filter",
    inputs: &["real"],
    options: &["period"],
    outputs: &["highpass"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "highpass",
        label: "Ehlers High Pass",
        display_type: DisplayType::Indicator,
        outputs: &["highpass"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: (f64, f64),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64)) -> Self {
        Self { multipliers, state }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut highpass_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            &mut self.state,
            self.multipliers,
            &mut highpass_line,
        );

        Ok(vec![highpass_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub y1: f64, // y[t-1]
    pub prev_real: f64,
}
impl State {
    pub fn new() -> Self {
        Self {
            y1: 0.0,
            prev_real: 0.0,
        }
    }
    pub fn init_state(real: &[f64], period: usize, multipliers: (f64, f64)) -> Self {
        let mut state = Self::new();
        for &value in real.iter().take(period) {
            state.calc(value, multipliers);
        }
        state
    }
    #[inline(always)]
    pub fn calc(&mut self, real: f64, multipliers: (f64, f64)) -> f64 {
        let (a1, a2) = multipliers;
        let y = a1.mul_add(self.y1, a2 * (real - self.prev_real));
        self.prev_real = real;
        self.y1 = y;
        y
    }
}

/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// SuperSmoother is a 2-pole IIR filter with fixed coefficients — accuracy is
/// not dependent on exponential smoothing decay, so this always delegates to
/// [`min_data`] regardless of `decimals`.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (`period`).
/// * `decimals` - The number of decimal places of accuracy required (unused).
///
/// # Returns
///
/// The minimum number of input bars needed for meaningful SuperSmoother output.
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Returns the minimum amount of data required for the SuperSmoother indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the SuperSmoother calculation (`period`).
///
/// # Returns
///
/// The minimum number of input bars required (`period + 1`).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Returns the number of output values produced by the SuperSmoother indicator
/// given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the SuperSmoother calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Ehlers Super Smoother indicator over the full input dataset.
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
/// * `outputs[0]` — `supersmoother` line
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; SuperSmoother has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `supersmoother` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);
    validate_inputs(inputs, min_data(options))?;

    let mut highpass_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };
    let mut state = State::init_state(inputs[0], period, multipliers);

    let real = &inputs[0][period..];
    cycle(real, &mut state, multipliers, &mut highpass_line);

    Ok((vec![highpass_line], IndicatorState::new(state, multipliers)))
}

/// Performs the core filter loop for the SuperSmoother indicator.
///
/// # Arguments
///
/// * `real` - A slice of input price values.
/// * `state` - A mutable reference to the filter state (`y1`, `y2`).
/// * `multipliers` - The precomputed filter coefficients `(a1, a2, b0)`.
/// * `super_line` - Output slice for the filtered values (must be the same length as `real`).
fn cycle(real: &[f64], state: &mut State, multipliers: (f64, f64), highpass_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *highpass_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i), multipliers);
        }
    }
}

/// Calculates the SuperSmoother value for a single bar.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current filter state (`y1`, `y2`).
/// * `real` - The current input price value.
/// * `multipliers` - The precomputed filter coefficients `(a1, a2, b0)`.
///
/// # Returns
///
/// The filtered output value for this bar.
#[inline(always)]
pub fn calc(state: &mut State, real: f64, multipliers: (f64, f64)) -> f64 {
    state.calc(real, multipliers)
}

/// Computes the 2-pole SuperSmoother filter coefficients for a given period.
///
/// # Arguments
///
/// * `period` - The filter period. Controls the cutoff frequency of the smoother.
///
/// # Returns
///
/// A tuple `(a1, a2, b0)` where:
/// - `a1`, `a2` are the IIR feedback coefficients
/// - `b0` is the feedforward gain (`1 - a1 - a2`)
pub fn multiplier(period: usize) -> (f64, f64) {
    let omega = std::f64::consts::TAU / period as f64;

    let a1 = (1.0 - omega.sin()) / omega.cos();
    let a2 = 0.5 * (1.0 + a1);

    (a1, a2)
}
