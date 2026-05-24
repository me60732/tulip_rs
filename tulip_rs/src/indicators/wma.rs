use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::sma::{calc as calc_sma, multiplier as sma_multiplier};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wma_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::wma_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::wma_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::wma_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    multipliers: (f64, f64, f64),
    state: State,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], multipliers: (f64, f64, f64), state: State, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            multipliers,
            state,
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

        let (mut wma_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_wma(
            &self.real,
            &mut self.state,
            self.period,
            self.multipliers,
            (&mut wma_line, &mut sma_line),
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![wma_line, sma_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub sum: f64,
    pub weighted_sum: f64,
}
impl State {
    pub fn new(sum: f64, weighted_sum: f64) -> Self {
        State { sum, weighted_sum }
    }
    pub fn init_state(prev_real: &[f64]) -> Self {
        let mut sum: f64 = 0.0;
        let mut weighted_sum: f64 = 0.0;

        for (i, &value) in prev_real.iter().enumerate() {
            sum += value;
            weighted_sum += value * (i + 1) as f64;
        }
        Self { sum, weighted_sum }
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        prev_value: &f64,
        value: &f64,
        multipliers: (f64, f64, f64),
    ) -> (f64, f64) {
        let (multiplier, weights, n) = multipliers;

        self.weighted_sum -= self.sum;

        let sma = calc_sma(&mut self.sum, value, prev_value, &multiplier);

        self.weighted_sum += value * n;

        let wma = self.weighted_sum / weights;

        (wma, sma)
    }
}
/// Returns information about the Weighted Moving Average (WMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the WMA indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "wma",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Weighted Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["wma"],
        optional_outputs: &["sma"],
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options: `[period]`.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the WMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the WMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the WMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Weighted Moving Average (WMA) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `real` (price series)
///
/// # Options
///
/// * `options[0]` — `period`
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to enable the optional `sma` output;
///   `None` disables it.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `wma` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut wma_line, mut sma_line) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                sma_line: capacity
            ),
        )
    };

    let mut state = State::init_state(&real[0..period]);

    cycle_wma(
        real,
        &mut state,
        period,
        multipliers,
        (&mut wma_line, &mut sma_line),
    );

    Ok((
        vec![wma_line, sma_line],
        IndicatorState::new(real, multipliers, state, period),
    ))
}

/// Performs the main calculation loop for the WMA indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - Mutable reference to the rolling `State` (sum and weighted sum).
/// * `period` - The period for the WMA calculation.
/// * `multipliers` - A tuple of `(sma_multiplier, weights, n)` from `multiplier()`.
/// * `out_vecs` - Mutable output slices: `(wma_line, sma_line)`.
fn cycle_wma(
    real: &[f64],
    state: &mut State,
    period: usize,
    multipliers: (f64, f64, f64),
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (wma_line, sma_line) = out_vecs;
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (wma, sma);
        unsafe {
            (wma, sma) = state.calc(real.get_unchecked(j), real.get_unchecked(i), multipliers);
            *wma_line.get_unchecked_mut(j) = wma;
        }
        crate::store_optional_outputs!(j,
            want_sma, sma_line => sma
        );
    }
}

/// Calculates the Weighted Moving Average (WMA) for the current data point using rolling sums.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State` (sum and weighted sum).
/// * `prev_value` - The value leaving the window (oldest element).
/// * `value` - The new value entering the window (newest element).
/// * `multipliers` - A tuple of `(sma_multiplier, weights, n)` from `multiplier()`.
///
/// # Returns
///
/// A tuple of `(wma, sma)` for this bar.
#[inline(always)]
pub fn calc(
    state: &mut State,
    prev_value: &f64,
    value: &f64,
    multipliers: (f64, f64, f64),
) -> (f64, f64) {
    state.calc(prev_value, value, multipliers)
}
#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64, f64) {
    let n = period as f64;
    let weights = n * (n + 1.0) / 2.0;
    (sma_multiplier(period), weights, n)
}
