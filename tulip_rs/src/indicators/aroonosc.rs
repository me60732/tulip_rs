use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::aroon::State;
use crate::indicators::aroon::{calc as calc_aroon, calc_unchecked as calc_unchecked_aroon};
pub use crate::indicators::aroon::{multiplier, OPTIONS_WIDTH};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::aroonosc_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
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
pub const INFO: Info = Info {
    name: "aroonosc",
    full_name: "Aroon Oscillator",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low"],
    options: &["period"],
    outputs: &["aroonosc"],
    optional_outputs: &["aroon_down", "aroon_up"],
    display_groups: &[
        DisplayGroup {
            id: "aroonosc",
            label: "AROONOSC",
            display_type: DisplayType::Indicator,
            outputs: &["aroonosc"],
        },
        DisplayGroup {
            id: "aroon_down_aroon_up",
            label: "Aroon",
            display_type: DisplayType::Indicator,
            outputs: &["aroon_down", "aroon_up"],
        }
    ],
};
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

/// Calculates the output length for the Aroon Oscillator indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Aroon Oscillator calculation.
///
/// # Returns
///
/// The number of output values produced by the Aroon Oscillator calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Aroon Oscillator indicator over the full input dataset.
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
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable `aroon_down`;
///   `Some(&[false, true])` to enable `aroon_up`; `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `aroonosc`, `outputs[1]` is `aroon_down`
/// (optional), and `outputs[2]` is `aroon_up` (optional). `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
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
/// * `inputs` - A tuple of high and low price slices.
/// * `period` - The period for the Aroon Oscillator calculation.
/// * `multiplier` - The multiplier used to scale Aroon values (100 / period).
/// * `aroonosc_line` - A mutable slice for storing the Aroon Oscillator values.
/// * `state` - A mutable reference to the current indicator state.
/// * `out_vecs` - A tuple of mutable slices for storing optional Aroon down and Aroon up lines.
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
