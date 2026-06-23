use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;

pub use crate::indicators::max::{min_data, output_length};
use crate::indicators::{
    max::{State as MaxState, CHUNK_1, CHUNK_4},
    medprice::calc as calc_medprice,
    min::State as MinState,
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::donchianchannel_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::donchianchannel_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::donchianchannel_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::donchianchannel_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
    periods: (usize, usize),
}
impl IndicatorState {
    /// Creates a new `IndicatorState` for streaming continuation.
    ///
    /// Retains the trailing `period - 1` high and low bars needed to maintain the sliding
    /// max/min windows across batch calls.
    ///
    /// # Arguments
    ///
    /// * `state` - Internal min/max state after the last computed bar.
    /// * `high` - Full high price slice from the just-completed batch.
    /// * `low` - Full low price slice from the just-completed batch.
    /// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
    pub fn new(state: State, high: &[f64], low: &[f64], periods: (usize, usize)) -> Self {
        Self {
            high: high[high.len() - periods.1..].to_vec(),
            low: low[low.len() - periods.1..].to_vec(),
            state,
            periods,
        }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);
        let (mut lower_line, mut middle_line, mut upper_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        if CHUNK_1.contains(&self.periods.0) {
            cycle::<1>(
                (&self.high, &self.low),
                self.periods,
                (&mut lower_line, &mut middle_line, &mut upper_line),
                &mut self.state,
            );
        } else if CHUNK_4.contains(&self.periods.0) {
            cycle::<4>(
                (&self.high, &self.low),
                self.periods,
                (&mut lower_line, &mut middle_line, &mut upper_line),
                &mut self.state,
            );
        } else {
            cycle::<8>(
                (&self.high, &self.low),
                self.periods,
                (&mut lower_line, &mut middle_line, &mut upper_line),
                &mut self.state,
            );
        }

        self.high.drain(..self.high.len() - self.periods.1);
        self.low.drain(..self.low.len() - self.periods.1);

        Ok(vec![lower_line, middle_line, upper_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
}
impl State {
    /// Initialises the Donchian Channel state from the seed bar.
    ///
    /// Seeds the min/max sliding windows at the first `low`/`high` value.
    ///
    /// # Arguments
    ///
    /// * `high` - High prices; must contain at least `period` elements.
    /// * `low` - Low prices; must contain at least `period` elements.
    /// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
    pub fn new(high: &[f64], low: &[f64], periods: (usize, usize)) -> Self {
        let min_state = MinState::new(low[0], periods.1);
        let max_state = MaxState::new(high[0], periods.1);
        State {
            min_state,
            max_state,
        }
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        inputs: (&[f64], &[f64]),
        i: usize,
        periods: (usize, usize),
    ) -> (f64, f64, f64) {
        let (high, low) = inputs;
        let (min, _) = self.min_state.calc(low, i, periods);
        let (max, _) = self.max_state.calc(high, i, periods);

        let middle = calc_medprice(max, min);

        (min, middle, max)
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
    /// * `state` - Mutable reference to the current min/max state.
    /// * `inputs` - A tuple of `(high_slice, low_slice)`.
    /// * `i` - Current bar index into the slices.
    /// * `periods` - Tuple `(period, trail)` where `trail = period - 1`.
    ///
    /// # Returns
    ///
    /// A tuple `(lower, middle, upper)` for the current bar.
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked<const N: usize>(
        &mut self,
        inputs: (&[f64], &[f64]),
        i: usize,
        periods: (usize, usize),
    ) -> (f64, f64, f64) {
        let (high, low) = inputs;
        let (min, _) = self.min_state.calc_unchecked::<N>(low, i, periods);
        let (max, _) = self.max_state.calc_unchecked::<N>(high, i, periods);

        let middle = calc_medprice(max, min);

        (min, middle, max)
    }
}
/// Returns information about the Donchian Channel indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Donchian Channel indicator.
pub const INFO: Info = Info {
    name: "donchianchannel",
    full_name: "Donchian Channel",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low"],
    options: &["period"],
    outputs: &["lower", "middle", "upper"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "donchianchannel",
        label: "Donchian Channel",
        display_type: DisplayType::Overlay,
        outputs: &["lower", "middle", "upper"],
    }],
};

/// Calculates the Donchian Channel indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period (highest-high / lowest-low lookback window)
///
/// # Outputs
///
/// * `outputs[0]` — `lower` band (lowest low over `period` bars)
/// * `outputs[1]` — `middle` band (`(upper + lower) / 2`)
/// * `outputs[2]` — `upper` band (highest high over `period` bars)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Unused; pass `None`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `lower`, `outputs[1]` is `middle`,
/// `outputs[2]` is `upper`, and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming continuation.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let periods = (options[0] as usize, options[0] as usize - 1);
    let [high, low] = inputs;

    let (mut lower_line, mut middle_line, mut upper_line) = {
        let capacity = output_length(high.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        )
    };

    let mut state = State::new(high, low, periods);

    if CHUNK_1.contains(&periods.0) {
        cycle::<1>(
            (high, low),
            periods,
            (&mut lower_line, &mut middle_line, &mut upper_line),
            &mut state,
        );
    } else if CHUNK_4.contains(&periods.0) {
        cycle::<4>(
            (high, low),
            periods,
            (&mut lower_line, &mut middle_line, &mut upper_line),
            &mut state,
        );
    } else {
        cycle::<8>(
            (high, low),
            periods,
            (&mut lower_line, &mut middle_line, &mut upper_line),
            &mut state,
        );
    }
    Ok((
        vec![lower_line, middle_line, upper_line],
        IndicatorState::new(state, high, low, periods),
    ))
}

/// Performs the main calculation loop for the Donchian Channel indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low)` price slices.
/// * `periods` - A tuple of `(period, trail)` where `trail = period - 1`.
/// * `output_lines` - A tuple of mutable slices for storing `(lower, middle, upper)`.
/// * `state` - A mutable reference to the current indicator state.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64]),
    periods: (usize, usize),
    output_lines: (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State,
) {
    let (lower_line, middle_line, upper_line) = output_lines;

    for (j, i) in (periods.1..inputs.0.len()).enumerate() {
        unsafe {
            let (lower, middle, upper) = state.calc_unchecked::<N>(inputs, i, periods);
            *lower_line.get_unchecked_mut(j) = lower;
            *middle_line.get_unchecked_mut(j) = middle;
            *upper_line.get_unchecked_mut(j) = upper;
        }
    }
}
