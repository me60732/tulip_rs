use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::{
    atr::{Atr, State as AtrState},
    max::{Max, State as MaxState},
    min::State as MinState,
    tr::Tr,
};

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

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
    state: State<Warm>,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(high: &[f64], low: &[f64], state: State<Warm>, periods: (usize, usize)) -> Self {
        Self {
            high: high[high.len() - periods.0..].to_vec(),
            low: low[low.len() - periods.0..].to_vec(),
            state,
            periods,
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

        cycle(
            (&self.high, &self.low, close),
            periods,
            (&mut long_line, &mut short_line),
            &mut self.state,
            (&mut atr_line, &mut tr_line, &mut min_line, &mut max_line),
        );

        self.high.drain(..self.high.len() - periods.0);
        self.low.drain(..self.low.len() - periods.0);

        Ok(vec![
            long_line, short_line, atr_line, tr_line, min_line, max_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub atr_state: AtrState<S>,
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
    pub step: f64,
}

impl State<Cold> {
    pub fn init_state(
        (high, low, close): (&[f64], &[f64], &[f64]),
        period: usize,
        trail: usize,
        step: f64,
        optional_outputs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (tr_line, min_line, max_line) = optional_outputs;
        let mut min_state = MinState::init_state(low, trail);
        let mut max_state = MaxState::init_state(high, trail);
        let max = max_state.calc((high, trail, (period, trail))).0;
        let min = min_state.calc((low, trail, (period, trail))).0;
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, false);

        crate::init_store_optional_outputs!(trail, high.len(),
            min_line => min,
            max_line => max
        );
        State {
            min_state,
            max_state,
            atr_state,
            step,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], f64, usize, (usize, usize));
    type Outputs = (f64, f64, f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, i, periods): Self::Inputs<'a>,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (min, _) = self.min_state.calc((low, i, periods));
        let (max, _) = self.max_state.calc((high, i, periods));

        let (atr, tr) = self.atr_state.calc((high[i], low[i], close));

        let long = atr.mul_add(-self.step, max);
        let short = atr.mul_add(self.step, min);

        (long, short, atr, tr, min, max)
    }
    #[inline(always)]
    unsafe fn calc_unchecked(
        &mut self,
        inputs: Self::Inputs<'_>,
    ) -> Self::Outputs {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (high, low, close, i, periods): (&[f64], &[f64], f64, usize, (usize, usize)),
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (min, _) = self.min_state.calc_chuncked_unchecked::<N>((low, i, periods));
        let (max, _) = self.max_state.calc_chuncked_unchecked::<N>((high, i, periods));

        let (atr, tr) = self
            .atr_state
            .calc((*high.get_unchecked(i), *low.get_unchecked(i), close));
        let long = atr.mul_add(-self.step, max);
        let short = atr.mul_add(self.step, min);
        (long, short, atr, tr, min, max)
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
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
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    periods: (usize, usize),
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State<Warm>,
    optional_outputs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (long_line, short_line) = output_lines;
    let (atr_line, tr_line, min_line, max_line) = optional_outputs;
    let (has_optional, want_atr, want_tr, want_min, want_max) =
        crate::calc_want_flags!(atr_line, tr_line, min_line, max_line);
    for (j, i) in (periods.0..inputs.0.len()).enumerate() {
        let (long, short, atr, tr, min, max);// =state.calc((high, low, unsafe { *close.get_unchecked(j) }, i, periods));
        unsafe {
            (long, short, atr, tr, min, max) =
                state.calc_unchecked((high, low, *close.get_unchecked(j), i, periods));
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

pub struct ChandelierExit;

impl Indicator<INPUTS, OPTIONS> for ChandelierExit {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
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

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let periods = (options[0] as usize, options[0] as usize - 1);
        let step = options[1];
        let [high, low, close] = inputs;

        let (
            mut long_line,
            mut short_line,
            (mut atr_line, mut tr_line, mut min_line, mut max_line),
        ) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            let min_max_capacity = Max::output_length(len, &[options[0]]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false, false],
                    atr_line: Atr::output_length(len, &[options[0]]),
                    tr_line: Tr::output_length(len, &[]),
                    min_line: min_max_capacity,
                    max_line: min_max_capacity
                ),
            )
        };

        let mut state = State::init_state(
            (high, low, close),
            periods.0,
            periods.1,
            step,
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

        cycle(
            (high, low, &close[periods.0..]),
            periods,
            (&mut long_line, &mut short_line),
            &mut state,
            optional_outputs,
        );
        
        Ok((
            vec![long_line, short_line, atr_line, tr_line, min_line, max_line],
            IndicatorState::new(high, low, state, periods),
        ))
    }
}
