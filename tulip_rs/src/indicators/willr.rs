use crate::common::{validate_inputs, validate_options};
//use crate::indicators::aroon::State;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::{
    calc as calc_max, calc_unchecked as calc_max_uncheked, State as MaxState,
};
use crate::indicators::min::{
    calc as calc_min, calc_unchecked as calc_min_uncheked, State as MinState,
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::willr_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::willr_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::willr_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::willr_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    high: Vec<f64>,
    low: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, high: &[f64], low: &[f64], period: usize) -> Self {
        Self {
            state,
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            period,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        // Merge stored tails with new inputs.
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let close = inputs[2];

        let mut willr_line: Vec<f64> = crate::uninit_vec!(f64, inputs[0].len());
        match self.period {
            1..=13 => {
                cycle_willr::<1>(
                    &self.high,
                    &self.low,
                    close,
                    self.period,
                    &mut self.state,
                    &mut willr_line,
                );
            }
            14..30 => {
                cycle_willr::<4>(
                    &self.high,
                    &self.low,
                    close,
                    self.period,
                    &mut self.state,
                    &mut willr_line,
                );
            }
            _ => {
                cycle_willr::<8>(
                    &self.high,
                    &self.low,
                    close,
                    self.period,
                    &mut self.state,
                    &mut willr_line,
                );
            }
        }

        self.high.drain(..self.high.len() - self.period);
        self.low.drain(..self.low.len() - self.period);

        Ok(vec![willr_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
}
impl State {
    pub fn new(min_state: (f64, usize), max_state: (f64, usize)) -> Self {
        State {
            min_state: MinState::new(min_state.0, min_state.1),
            max_state: MaxState::new(max_state.0, max_state.1),
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> Self {
        let mut state = State::new((low[0], period - 1), (high[0], period - 1));

        _ = calc_min(&mut state.min_state, low, period - 1, (period, period - 1));
        _ = calc_max(&mut state.max_state, high, period - 1, (period, period - 1));

        state
    }
}
pub const INFO: Info = Info {
    name: "willr",
    full_name: "Williams %R",
    indicator_type: IndicatorType::Momentum,
    // Three inputs: high, low, close.
    inputs: &["high", "low", "close"],
    // One option: period.
    options: &["period"],
    outputs: &["willr"],
    optional_outputs: &[],
    display_groups: &[
        DisplayGroup {
            id: "willr",
            label: "WILLR",
            display_type: DisplayType::Indicator,
            outputs: &["willr"],
        },
    ],
};
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
/// Returns the minimum amount of data required for the Williams %R indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[period]`.
///
/// # Returns
///
/// The minimum amount of data required (period + 1).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Williams %R calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Williams %R indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `high`
/// * `inputs[1]` — `low`
/// * `inputs[2]` — `close`
///
/// # Options
///
/// * `options[0]` — `period`
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `willr` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut willr_line = {
        let capacity = output_length(close.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(high, low, period);

    // The first valid calculation is at index period - 1 within the slice.
    match period {
        1..=13 => {
            cycle_willr::<1>(
                high,
                low,
                &close[period..],
                period,
                &mut state,
                &mut willr_line,
            );
        }
        14..25 => {
            cycle_willr::<4>(
                high,
                low,
                &close[period..],
                period,
                &mut state,
                &mut willr_line,
            );
        }
        _ => {
            cycle_willr::<8>(
                high,
                low,
                &close[period..],
                period,
                &mut state,
                &mut willr_line,
            );
        }
    }

    Ok((
        vec![willr_line],
        IndicatorState::new(state, high, low, period),
    ))
}

/// Iterates over the high, low, and close slices and computes Williams %R values.
///
/// # Arguments
///
/// * `high` - The full high price input slice.
/// * `low` - The full low price input slice.
/// * `close` - The close price slice to iterate over (already offset by `period`).
/// * `period` - The lookback period.
/// * `state` - Mutable reference to the rolling `State` (min and max states).
/// * `willr_line` - Mutable output slice for Williams %R values.
fn cycle_willr<const N: usize>(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    state: &mut State,
    willr_line: &mut [f64],
) {
    //let shift = low.len() - close.len();
    let periods = (period, period - 1);
    let mut i = period;
    for (close, willr) in close.iter().zip(willr_line.iter_mut()) {
        unsafe {
            *willr = calc_unchecked::<N>(state, high, low, close, i, periods);
        }
        i += 1;
    }
}

/// Calculates WillR for a single bar using the sliding window state.
/// It mimics stoch’s calc_kfast but uses the WillR formula:
/// willr = -100 * (max - close[i]) / (max - min)
#[inline(always)]
pub fn calc(
    state: &mut State,
    high: &[f64],
    low: &[f64],
    close: &f64,
    i: usize,
    periods: (usize, usize),
) -> f64 {
    // Update the minimum and maximum for the rolling window.
    let (min, _) = calc_min(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max(&mut state.max_state, high, i, periods);

    if (max - min).abs() < f64::EPSILON {
        return 0.0;
    }

    100.0 * (max - close) / (max - min)
}
/// Calculates Williams %R for a single bar using unchecked min/max access.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State` (min and max states).
/// * `high` - The full high price input slice.
/// * `low` - The full low price input slice.
/// * `close` - Reference to the current bar's close price.
/// * `i` - The current index into `high` and `low`.
/// * `periods` - A tuple of `(period, period - 1)` used by the min/max states.
///
/// # Returns
///
/// The Williams %R value for this bar.
///
/// # Safety
///
/// `i` and the look-back window must be within bounds of `high` and `low`.
#[inline(always)]
pub unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    high: &[f64],
    low: &[f64],
    close: &f64,
    i: usize,
    periods: (usize, usize),
) -> f64 {
    // Update the minimum and maximum for the rolling window.
    let (min, _) = calc_min_uncheked::<N>(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max_uncheked::<N>(&mut state.max_state, high, i, periods);

    if (max - min).abs() < f64::EPSILON {
        return 0.0;
    }
    100.0 * (max - close) / (max - min)
}
