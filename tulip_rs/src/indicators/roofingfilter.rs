//! # Ehlers Roofing Filter
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 2.
//!
//! A band-pass pre-filter that cascades the High Pass and Super Smoother filters
//! to band-limit a price signal to the cycle frequencies of interest. The name
//! comes from the idea that the combined filter creates a "roof" at the high end
//! (via the Super Smoother) and a "floor" at the low end (via the High Pass),
//! confining the output to a specific frequency band.
//!
//! ## Pipeline
//!
//! ```text
//! Price
//!   │
//!   ▼
//! High Pass filter  (cutoff = hp_period bars)   removes DC trend / long-cycle drift
//!   │
//!   ▼
//! Super Smoother    (cutoff = ss_period bars)   removes high-frequency noise / aliasing
//!   │
//!   ▼
//! Roofed signal  (band-limited to [ss_period, hp_period] bar cycles)
//! ```
//!
//! Options: `[ss_period, hp_period]`.  A typical configuration is
//! `ss_period = 10, hp_period = 48`, preserving 10–48 bar cycles.
//!
//! ## Role in this library
//!
//! Used as the first stage of [`hilberttransform`], which applies the
//! Hilbert kernel to the roofed (band-limited) signal rather than to raw price.
//! This is the key architectural difference between our Hilbert Transform
//! (Ehlers 2013) and TA-Lib's `HT_PHASOR` (Ehlers 2001), which applies the
//! kernel directly to a simple WMA-smoothed price.


use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{
    highpass::{multiplier as hp_multiplier, output_length as hp_outputlength, State as HpState},
    supersmoother::{multiplier as ss_multiplier, State as SsState},
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};

use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_options as indicator;
}

/// Returns metadata for the Ehlers Roofing Filter indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the RoofingFilter indicator, including
/// its input (`real`), configurable `ss_period` and `hp_period`, primary output
/// (`roofing`), and optional output (`highpass`).
pub const INFO: Info = Info {
    name: "roofingfilter",
    indicator_type: IndicatorType::Trend,
    full_name: "Ehlers Roofing Filter",
    inputs: &["real"],
    options: &["ss_period, hp_period"],
    outputs: &["roofing"],
    optional_outputs: &["highpass"],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "roofing",
        label: "Ehlers Roofing Filter",
        display_type: DisplayType::Indicator,
        outputs: &["roofing", "highpass"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64, f64), (f64, f64)),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, multipliers: ((f64, f64, f64), (f64, f64))) -> Self {
        Self { multipliers, state }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut rf_line, mut hp_line) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    hp_line: len
                ),
            )
        };

        cycle(
            inputs[0],
            &mut self.state,
            self.multipliers,
            (&mut rf_line, &mut hp_line),
        );

        Ok(vec![rf_line, hp_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub ss_state: SsState, // SuperSmoother (low-pass) state
    pub hp_state: HpState,
}
impl State {
    pub fn new() -> Self {
        Self {
            ss_state: SsState::new(),
            hp_state: HpState::new(),
        }
    }
    pub fn init_state(
        real: &[f64],
        periods: (usize, usize),
        multipliers: ((f64, f64, f64), (f64, f64)),
        hp_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new();
        let l_period = periods.0.max(periods.1);
        for (i, &value) in real.iter().take(l_period).enumerate() {
            let (_, hp) = state.calc(value, multipliers);
            crate::init_store_optional_outputs!(i, real.len(),
                hp_line => hp
            );
        }
        state
    }
    #[inline(always)]
    pub fn calc(&mut self, real: f64, multipliers: ((f64, f64, f64), (f64, f64))) -> (f64, f64) {
        let hp = self.hp_state.calc(real, multipliers.1);
        (self.ss_state.calc(hp, multipliers.0), hp)
    }
}

/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// RoofingFilter is an IIR cascade with fixed coefficients — accuracy is
/// not dependent on exponential smoothing decay, so this always delegates to
/// [`min_data`] regardless of `decimals`.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (`ss_period`, `hp_period`).
/// * `decimals` - The number of decimal places of accuracy required (unused).
///
/// # Returns
///
/// The minimum number of input bars needed for meaningful RoofingFilter output.
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Returns the minimum amount of data required for the RoofingFilter indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[ss_period, hp_period]`.
///
/// # Returns
///
/// The minimum number of input bars required (`max(ss_period, hp_period) + 1`).
pub fn min_data(options: &[f64]) -> usize {
    options[0].max(options[1]) as usize + 1
}

/// Returns the number of output values produced by the RoofingFilter indicator
/// given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options: `[ss_period, hp_period]`.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Ehlers Roofing Filter over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — `ss_period` (SuperSmoother period, low-pass cutoff)
/// * `options[1]` — `hp_period` (HighPass period, high-pass cutoff)
///
/// # Outputs
///
/// * `outputs[0]` — `roofing` line
///
/// # Optional Outputs
///
/// * `outputs[1]` — `highpass` line (intermediate high-pass output; enabled via `optional_outputs`)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to also emit the `highpass` line.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `roofing` line,
/// `outputs[1]` is the optional `highpass` line (empty if not requested), and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let periods = (options[0] as usize, options[1] as usize);
    let multipliers = multiplier(periods);
    validate_inputs(inputs, min_data(options))?;

    let (mut rf_line, mut hp_line) = {
        let capacity = output_length(inputs[0].len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                hp_line: hp_outputlength(inputs[0].len(), &[periods.1 as f64])
            ),
        )
    };
    let mut state = State::init_state(inputs[0], periods, multipliers, &mut hp_line);

    let real = &inputs[0][periods.0.max(periods.1)..];
    let outputs = {
        let offset = crate::slice_outputs_start!(rf_line.len(), hp_line);
        (rf_line.as_mut_slice(), &mut hp_line[offset..])
    };
    cycle(real, &mut state, multipliers, outputs);

    Ok((
        vec![rf_line, hp_line],
        IndicatorState::new(state, multipliers),
    ))
}

/// Performs the core filter loop for the RoofingFilter indicator.
///
/// # Arguments
///
/// * `real` - A slice of input price values.
/// * `state` - A mutable reference to the composite filter state (`ss_state`, `hp_state`).
/// * `multipliers` - The precomputed filter coefficients `((a1, a2, b0), (a1, a2))`.
/// * `outputs` - Tuple of `(rf_line, hp_line)` output slices; `rf_line` must be the same length as `real`.
fn cycle(
    real: &[f64],
    state: &mut State,
    multipliers: ((f64, f64, f64), (f64, f64)),
    outputs: (&mut [f64], &mut [f64]),
) {
    let (rf_line, hp_line) = outputs;
    let (_, want_hp) = crate::calc_want_flags!(hp_line);
    for i in 0..real.len() {
        let (rf, hp);
        unsafe {
            (rf, hp) = state.calc(*real.get_unchecked(i), multipliers);
            *rf_line.get_unchecked_mut(i) = rf;
        }
        crate::store_optional_outputs!(i,
            want_hp, hp_line => hp
        );
    }
}

/// Calculates the RoofingFilter values for a single bar.
///
/// # Arguments
///
/// * `state` - A mutable reference to the composite filter state (`ss_state`, `hp_state`).
/// * `real` - The current input price value.
/// * `multipliers` - The precomputed filter coefficients `((a1, a2, b0), (a1, a2))`.
///
/// # Returns
///
/// A tuple `(roofing, highpass)` for this bar.
#[inline(always)]
pub fn calc(
    state: &mut State,
    real: f64,
    multipliers: ((f64, f64, f64), (f64, f64)),
) -> (f64, f64) {
    state.calc(real, multipliers)
}

/// Computes the RoofingFilter coefficients for both sub-filters.
///
/// # Arguments
///
/// * `periods` - Tuple of `(ss_period, hp_period)`.
///
/// # Returns
///
/// A tuple `((a1, a2, b0), (a1, a2))` where:
/// - `(a1, a2, b0)` are the SuperSmoother (low-pass) coefficients
/// - `(a1, a2)` are the HighPass coefficients
pub fn multiplier(periods: (usize, usize)) -> ((f64, f64, f64), (f64, f64)) {
    (ss_multiplier(periods.0), hp_multiplier(periods.1))
}
