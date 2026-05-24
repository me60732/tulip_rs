use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::psar_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::psar_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::psar_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::psar_simd::indicator_by_options as indicator;
}

/// Returns information about the Parabolic SAR (PSAR) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the PSAR indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "psar",
        full_name: "Parabolic SAR",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low"],
        options: &["acceleration_factor", "max_acceleration_factor"],
        outputs: &["psar"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    high: Vec<f64>,
    low: Vec<f64>,
    options: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, high: &[f64], low: &[f64], options: (f64, f64)) -> Self {
        Self {
            state,
            options,
            high: high[high.len() - 2..].to_vec(),
            low: low[low.len() - 2..].to_vec(),
        }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let mut psar_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_psar(
            (&self.high, &self.low),
            &mut psar_line,
            self.options,
            &mut self.state,
            2,
        );
        self.high.drain(..self.high.len() - 2);
        self.low.drain(..self.low.len() - 2);
        Ok(vec![psar_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub psar: f64,
    pub extream: f64,
    pub accel: f64,
    pub uptrend: bool,
}
impl State {
    pub fn new(high: &[f64], low: &[f64], af_step: f64) -> Self {
        let (uptrend, extream, psar) = if high[0] + low[0] <= high[1] + low[1] {
            (true, high[0], low[0])
        } else {
            (false, low[0], high[0])
        };
        State {
            psar,
            extream,
            uptrend,
            accel: af_step,
        }
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value regardless of `_decimals`. Returns
/// one more than [`min_data`] to allow the algorithm to warm up past its
/// initial seed bar.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// `min_data(options) + 1` — the minimum number of input bars needed.
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options) + 1
}
/// Returns the minimum amount of data required for the PSAR indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the PSAR calculation (unused).
///
/// # Returns
///
/// The minimum number of input data points required (`2`).
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Returns the output length for the PSAR indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the PSAR calculation (unused for length).
///
/// # Returns
///
/// The number of output values produced.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len - min_data(_options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] <= 0.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Parabolic SAR (PSAR) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — acceleration_factor (initial step and per-bar increment)
/// * `options[1]` — max_acceleration_factor (upper bound on the acceleration factor)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `psar` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let af_step = options[0];
    let max_af = options[1];

    validate_inputs(inputs, min_data(options))?;

    let high = inputs[0];
    let low = inputs[1];

    let mut state = State::new(high, low, af_step);
    let mut psar_line = {
        let capacity = output_length(high.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle_psar(
        (high, low),
        &mut psar_line,
        (af_step, max_af),
        &mut state,
        1,
    );

    Ok((
        vec![psar_line],
        IndicatorState::new(state, high, low, (af_step, max_af)),
    ))
}

/// Iterates over the input data and applies the calc function.
fn cycle_psar(
    inputs: (&[f64], &[f64]),
    psar_line: &mut [f64],
    options: (f64, f64),
    state: &mut State,
    start: usize,
) {
    let (af_step, max_af) = options;
    let (high, low) = inputs;

    for (j, i) in (start..high.len()).enumerate() {
        unsafe {
            *psar_line.get_unchecked_mut(j) = calc_unchecked(state, high, low, af_step, max_af, i);
        }
    }
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    high: &[f64],
    low: &[f64],
    af_step: f64,
    max_af: f64,
    i: usize,
) -> f64 {
    let (mut psar, mut extream, mut uptrend, mut accel) =
        (state.psar, state.extream, state.uptrend, state.accel);

    // Use += for potential FMA optimization
    //psar += (extream - psar) * accel;
    psar = accel.mul_add(extream - psar, psar);
    if uptrend {
        // Keep original branch structure for better prediction
        if i >= 2 && psar > low[i - 2] {
            psar = low[i - 2];
        }
        if psar > low[i - 1] {
            psar = low[i - 1];
        }

        // Combined condition for extreme and acceleration
        if high[i] > extream {
            extream = high[i];
            accel = (accel + af_step).min(max_af);
        }
    } else {
        if i >= 2 && psar < high[i - 2] {
            psar = high[i - 2];
        }
        if psar < high[i - 1] {
            psar = high[i - 1];
        }

        if low[i] < extream {
            extream = low[i];
            accel = (accel + af_step).min(max_af);
        }
    }

    if (uptrend && low[i] < psar) || (!uptrend && high[i] > psar) {
        uptrend = !uptrend;
        psar = extream;
        accel = af_step;
        extream = if uptrend { high[i] } else { low[i] };
    }

    (state.psar, state.extream, state.uptrend, state.accel) = (psar, extream, uptrend, accel);
    psar
}

#[inline(always)]
pub unsafe fn calc_unchecked(
    state: &mut State,
    high: &[f64],
    low: &[f64],
    af_step: f64,
    max_af: f64,
    i: usize,
) -> f64 {
    let (mut psar, mut extream, mut uptrend, mut accel) =
        (state.psar, state.extream, state.uptrend, state.accel);
    let (h, prev_high, old_high) = (
        *high.get_unchecked(i),
        *high.get_unchecked(i - 1),
        if i > 1 {
            *high.get_unchecked(i - 2)
        } else {
            0.0
        },
    );
    let (l, prev_low, old_low) = (
        *low.get_unchecked(i),
        *low.get_unchecked(i - 1),
        if i > 1 {
            *low.get_unchecked(i - 2)
        } else {
            f64::MAX
        },
    );

    //psar += (extream - psar) * accel;
    psar = accel.mul_add(extream - psar, psar);
    if uptrend {
        if psar > old_low {
            psar = old_low;
        }
        if psar > prev_low {
            psar = prev_low;
        }

        if h > extream {
            extream = h;
            accel = (accel + af_step).min(max_af);
        }
    } else {
        if psar < old_high {
            psar = old_high;
        }
        if psar < prev_high {
            psar = prev_high;
        }

        if l < extream {
            extream = l;
            accel = (accel + af_step).min(max_af);
        }
    }

    if (uptrend && l < psar) || (!uptrend && h > psar) {
        uptrend = !uptrend;
        psar = extream;
        accel = af_step;
        extream = if uptrend { h } else { l };
    }

    (state.psar, state.extream, state.uptrend, state.accel) = (psar, extream, uptrend, accel);
    psar
}
