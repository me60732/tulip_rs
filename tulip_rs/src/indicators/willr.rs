use crate::common::{validate_inputs, validate_options};
//use crate::indicators::aroon::State;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::max::{Max, State as MaxState};
use crate::indicators::min::State as MinState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

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
    state: State<Warm>,
    high: Vec<f64>,
    low: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, high: &[f64], low: &[f64], period: usize) -> Self {
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
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        // Merge stored tails with new inputs.
        let [high, low, close] = *inputs;
        self.high.extend_from_slice(high);
        self.low.extend_from_slice(low);

        let (mut willr_line, (mut min_line, mut max_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    min_line: len,
                    max_line: len
                ),
            )
        };
        cycle_willr(
            (&self.high, &self.low, close),
            self.period,
            &mut self.state,
            &mut willr_line,
            (&mut min_line, &mut max_line),
        );
        
        self.high.drain(..self.high.len() - self.period);
        self.low.drain(..self.low.len() - self.period);

        Ok(vec![willr_line, min_line, max_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
}

impl State {
    pub fn new(min_state: (f64, usize), max_state: (f64, usize)) -> Self {
        State {
            min_state: MinState::new(min_state.0, min_state.1),
            max_state: MaxState::new(max_state.0, max_state.1),
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        min_max: (&mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (min_line, max_line) = min_max;
        let look_back = period -1;
        let mut min_state = MinState::init_state(low, look_back);
        let mut max_state = MaxState::init_state(high, look_back);

        let min = min_state.calc((low, look_back, (period, look_back))).0;
        let max = max_state.calc((high, look_back, (period, look_back))).0;
        crate::init_store_optional_outputs!(look_back, high.len(),
            min_line => min,
            max_line => max
        );
        State {
            min_state,
            max_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a[f64], &'a[f64], f64, usize, (usize, usize));
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, i, periods): Self::Inputs<'a>
    ) -> Self::Outputs {
        // Update the minimum and maximum for the rolling window.
        let (min, _) = self.min_state.calc((low, i, periods));
        let (max, _) = self.max_state.calc((high, i, periods));

        if (max - min).abs() < f64::EPSILON {
            return (0.0, min, max);
        }

        (100.0 * (max - close) / (max - min), min, max)
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
    unsafe fn calc_unchecked(
        &mut self,
        inputs: (&[f64], &[f64], f64, usize, (usize, usize))
    ) -> (f64, f64, f64) {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
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
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (high, low, close, i, periods): (&[f64], &[f64], f64, usize, (usize, usize))
    ) -> (f64, f64, f64) {
        // Update the minimum and maximum for the rolling window.
        let (min, _) = self.min_state.calc_chuncked_unchecked::<N>((low, i, periods));
        let (max, _) = self.max_state.calc_chuncked_unchecked::<N>((high, i, periods));

        if (max - min).abs() < f64::EPSILON {
            return (0.0, min, max);
        }
        (100.0 * (max - close) / (max - min), min, max)
    }
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
fn cycle_willr(
    (high, low, close): (&[f64], &[f64], &[f64]),
    period: usize,
    state: &mut State<Warm>,
    willr_line: &mut [f64],
    (min_line, max_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_min, want_max) = crate::calc_want_flags!(min_line, max_line);

    let periods = (period, period - 1);
    let mut i = period;
    for (j, (&close, willr)) in close.iter().zip(willr_line.iter_mut()).enumerate() {
        let (min, max);
        unsafe {
            (*willr, min, max) = state.calc_unchecked((high, low, close, i, periods));
        }

        if has_optional {
            crate::store_optional_outputs!(j,
                want_min, min_line => min,
                want_max, max_line => max
            );
        }

        i += 1;
    }
}

pub struct Willr;

impl Indicator<INPUTS, OPTIONS> for Willr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "willr",
        full_name: "Williams %R",
        indicator_type: IndicatorType::Momentum,
        // Three inputs: high, low, close.
        inputs: &["high", "low", "close"],
        // One option: period.
        options: &["period"],
        outputs: &["willr"],
        optional_outputs: &["min", "max"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "willr",
                label: "WILLR",
                display_type: DisplayType::Indicator,
                outputs: &["willr"],
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

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let (mut willr_line, (mut min_line, mut max_line)) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            let min_max_capacity = Max::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    min_line: min_max_capacity,
                    max_line: min_max_capacity
                ),
            )
        };

        let mut state = State::init_state(high, low, period, (&mut min_line, &mut max_line));
        let optional_outputs = {
            let (min_offset, max_offset) =
                crate::slice_outputs_start!(willr_line.len(), min_line, max_line);
            (&mut min_line[min_offset..], &mut max_line[max_offset..])
        };

        cycle_willr(
            (high, low, &close[period..]),
            period,
            &mut state,
            &mut willr_line,
            optional_outputs,
        );
        

        Ok((
            vec![willr_line, min_line, max_line],
            IndicatorState::new(state, high, low, period),
        ))
    }
}
