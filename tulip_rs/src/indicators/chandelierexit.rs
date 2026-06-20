use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;

pub use crate::indicators::atr::multiplier;
use crate::indicators::{
    atr::{output_length as atr_output_length, State as AtrState},
    max::{
        calc as calc_max, calc_unchecked as calc_max_unchecked, output_length as max_output_length,
        State as MaxState,
    },
    min::{calc as calc_min, calc_unchecked as calc_min_unchecked, State as MinState},
    tr::output_length as tr_output_length,
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::chandelierexit_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::chandelierexit_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::chandelierexit_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::chandelierexit_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
}
impl IndicatorState {
    /// Creates a new `IndicatorState` for streaming continuation.
    ///
    /// Retains the trailing `period` high and low bars needed to maintain the sliding
    /// max/min windows across batch calls.
    ///
    /// # Arguments
    ///
    /// * `high` - Full high price slice from the just-completed batch.
    /// * `low` - Full low price slice from the just-completed batch.
    /// * `state` - Internal min/max and ATR state after the last computed bar.
    /// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
    /// * `multipliers` - Tuple `(step, (atr_alpha, atr_1m_alpha))` used in `calc`.
    pub fn new(
        high: &[f64],
        low: &[f64],
        state: State,
        periods: (usize, usize),
        multipliers: (f64, (f64, f64)),
    ) -> Self {
        Self {
            high: high[high.len() - periods.0..].to_vec(),
            low: low[low.len() - periods.0..].to_vec(),
            state,
            periods,
            multipliers,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let periods = self.periods;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);
        let close = inputs[2];
        let (
            mut long_line,
            mut short_line,
            (mut atr_line, mut tr_line, mut min_line, mut max_line),
        ) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false, false],
                    atr_line: capacity,
                    tr_line: capacity,
                    min_line: capacity,
                    max_line: capacity
                ),
            )
        };
        match periods.0 {
            1..=4 => {
                cycle::<1>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line, &mut min_line, &mut max_line),
                );
            }
            5..25 => {
                cycle::<4>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line, &mut min_line, &mut max_line),
                );
            }
            _ => {
                cycle::<8>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line, &mut min_line, &mut max_line),
                );
            }
        }

        self.high.drain(..self.high.len() - periods.0);
        self.low.drain(..self.low.len() - periods.0);

        Ok(vec![
            long_line, short_line, atr_line, tr_line, min_line, max_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
    pub atr_state: AtrState,
}
impl State {
    /// Initialises the Chandelier Exit state from the seed bars.
    ///
    /// Starts the min/max windows at the first `low`/`high` value and seeds the ATR
    /// using a Wilder SMA over the first `period` bars.
    ///
    /// # Arguments
    ///
    /// * `high` - High prices; must contain at least `period` elements.
    /// * `low` - Low prices; must contain at least `period` elements.
    /// * `close` - Close prices; must contain at least `period` elements.
    /// * `period` - ATR lookback (Wilder smoothing period).
    /// * `trail` - Min/max sliding-window size (`= period - 1`).
    /// * `optional_outputs` - Output buffers `(tr_line, min_line, max_line)` written during warm-up; pass empty slices if not needed.
    pub fn new(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        trail: usize,
        optional_outputs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> Self {
        let (tr_line, min_line, max_line) = optional_outputs;
        let min_state = MinState::init_state(low, period, trail, min_line);
        let max_state = MaxState::init_state(high, period, trail, max_line);
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, false);

        State {
            min_state,
            max_state,
            atr_state,
        }
    }
}
/// Returns information about the Chandelier Exit indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Chandelier Exit indicator.
pub const INFO: Info = Info {
    name: "chandelierexit",
    full_name: "Chandelier Exit",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["period", "step"],
    outputs: &["long", "short"],
    optional_outputs: &["atr", "tr", "min", "max"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "long_short",
            label: "Exit Positions",
            display_type: DisplayType::Overlay,
            outputs: &["long", "short"],
        },
        DisplayGroup {
            offset: None,
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        },
        DisplayGroup {
            offset: None,
            id: "min_max",
            label: "Min & Max",
            display_type: DisplayType::Overlay,
            outputs: &["min", "max"],
        },
    ],
};
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Returns the minimum amount of data required for the Chandelier Exit indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Chandelier Exit calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the Chandelier Exit indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Chandelier Exit calculation.
///
/// # Returns
///
/// The number of output values produced by the Chandelier Exit calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Chandelier Exit indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (ATR and highest-high / lowest-low lookback)
/// * `options[1]` — multiplier applied to ATR for the exit distance
///
/// # Outputs
///
/// * `outputs[0]` — `long` exit line  (`highest_high - multiplier × ATR`)
/// * `outputs[1]` — `short` exit line (`lowest_low  + multiplier × ATR`)
///
/// # Optional Outputs
///
/// * `atr` — the Wilder ATR series used in the calculation
/// * `tr`  — the True Range series
/// * `min` — the rolling lowest-low series (same window as the short exit)
/// * `max` — the rolling highest-high series (same window as the long exit)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional flags `[want_atr, want_tr, want_min, want_max]` to enable extra outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `long`, `outputs[1]` is `short`,
/// `outputs[2]` is `atr` (empty unless requested), `outputs[3]` is `tr` (empty unless
/// requested), `outputs[4]` is `min` (empty unless requested), `outputs[5]` is `max`
/// (empty unless requested), and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let periods = (options[0] as usize, options[0] as usize - 1);
    let multipliers = (options[1], multiplier(periods.0));
    let [high, low, close] = inputs;

    let (mut long_line, mut short_line, (mut atr_line, mut tr_line, mut min_line, mut max_line)) = {
        let len = high.len();
        let capacity = output_length(len, options);
        let min_max_capacity = max_output_length(len, options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                atr_line: atr_output_length(len, &[options[0]]),
                tr_line: tr_output_length(len, &[]),
                min_line: min_max_capacity,
                max_line: min_max_capacity
            ),
        )
    };

    let mut state = State::new(
        high,
        low,
        close,
        periods.0,
        periods.1,
        (&mut tr_line, &mut min_line, &mut max_line),
    );
    let optional_outputs = {
        let (tr_offset, min_offset, max_offset) =
            crate::slice_outputs_start!(long_line.len(), tr_line, min_line, max_line);
        (
            atr_line.as_mut_slice(),
            &mut tr_line[tr_offset..],
            &mut min_line[min_offset..],
            &mut max_line[max_offset..],
        )
    };
    match periods.0 {
        1..=10 => {
            cycle::<1>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                optional_outputs,
            );
        }
        11..=25 => {
            cycle::<4>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                optional_outputs,
            );
        }
        _ => {
            cycle::<8>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                optional_outputs,
            );
        }
    }
    Ok((
        vec![long_line, short_line, atr_line, tr_line, min_line, max_line],
        IndicatorState::new(high, low, state, periods, multipliers),
    ))
}

/// Performs the main calculation loop for the Chandelier Exit indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `periods` - A tuple of `(period, trail)` where `period` is the ATR lookback and
///   `trail` (`= period - 1`) is the sliding window size passed to min/max states.
/// * `multipliers` - A tuple of `(step, atr_multipliers)` where `step` is the ATR multiplier
///   option and `atr_multipliers` are the Wilder smoothing constants.
/// * `output_lines` - A tuple of mutable slices for storing the `long` and `short` exit lines.
/// * `state` - A mutable reference to the current indicator state.
/// * `optional_outputs` - A tuple of mutable slices for optional `atr`, `tr`, `min`, and `max` outputs.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64], &[f64]),
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State,
    optional_outputs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (long_line, short_line) = output_lines;
    let (atr_line, tr_line, min_line, max_line) = optional_outputs;
    let (has_optional, want_atr, want_tr, want_min, want_max) =
        crate::calc_want_flags!(atr_line, tr_line, min_line, max_line);
    for (j, i) in (periods.0..inputs.0.len()).enumerate() {
        let (long, short, atr, tr, min, max);
        unsafe {
            (long, short, atr, tr, min, max) = calc_unchecked::<N>(
                state,
                (high, low, *close.get_unchecked(j)),
                i,
                periods,
                multipliers,
            );
            *long_line.get_unchecked_mut(j) = long;
            *short_line.get_unchecked_mut(j) = short;
        }
        if has_optional {
            crate::store_optional_outputs!(j,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr,
                want_min, min_line => min,
                want_max, max_line => max
            );
        }
    }
}
/// Computes one step of the Chandelier Exit indicator.
///
/// Advances the rolling min, max, and ATR states by one bar and returns the long and short
/// exit lines along with the current ATR and True Range values.
///
/// # Arguments
///
/// * `state` - Mutable reference to the current min/max/ATR state.
/// * `inputs` - A tuple of `(high_slice, low_slice, close)` where `close` is the scalar
///   close for the current bar and `high_slice`/`low_slice` cover the full lookback window.
/// * `i` - Current bar index into `high_slice`/`low_slice`.
/// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
/// * `multipliers` - Tuple `(step, (atr_alpha, atr_1m_alpha))`.
///
/// # Returns
///
/// A tuple `(long, short, atr, tr)` for the current bar.
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64], f64),
    i: usize,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
) -> (f64, f64, f64, f64, f64, f64) {
    let (high, low, close) = inputs;
    let (step, atr_multipliers) = multipliers;
    let (min, _) = calc_min(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max(&mut state.max_state, high, i, periods);

    let (atr, tr) = state
        .atr_state
        .calc(high[i], low[i], close, atr_multipliers);

    let long = atr.mul_add(-step, max);
    let short = atr.mul_add(step, min);

    (long, short, atr, tr, min, max)
}
/// Unchecked version of [`calc`] that uses SIMD-hint size `N` for the min/max windows.
///
/// Identical to [`calc`] but uses `get_unchecked` for all slice accesses and passes the
/// const generic `N` as a prefetch/SIMD-hint to the min/max helpers.
///
/// # Safety
///
/// Callers must ensure that `i` is a valid index into `high` and `low` and that the
/// slice lengths are sufficient for the lookback window (`trail + 1` elements before `i`).
///
/// # Arguments
///
/// * `state` - Mutable reference to the current min/max/ATR state.
/// * `inputs` - A tuple of `(high_slice, low_slice, close)` where `close` is the scalar
///   close for the current bar.
/// * `i` - Current bar index into `high_slice`/`low_slice`.
/// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
/// * `multipliers` - Tuple `(step, (atr_alpha, atr_1m_alpha))`.
///
/// # Returns
///
/// A tuple `(long, short, atr, tr)` for the current bar.
#[inline(always)]
pub(crate) unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    inputs: (&[f64], &[f64], f64),
    i: usize,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
) -> (f64, f64, f64, f64, f64, f64) {
    let (high, low, close) = inputs;
    let (step, atr_multipliers) = multipliers;
    let (min, _) = calc_min_unchecked::<N>(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max_unchecked::<N>(&mut state.max_state, high, i, periods);

    let (atr, tr) = state.atr_state.calc(
        *high.get_unchecked(i),
        *low.get_unchecked(i),
        close,
        atr_multipliers,
    );
    let long = atr.mul_add(-step, max);
    let short = atr.mul_add(step, min);
    (long, short, atr, tr, min, max)
}
