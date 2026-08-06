use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

pub use crate::indicators::max::Max;
use crate::indicators::{
    max::State as MaxState,
    medprice::calc as calc_medprice,
    min::State as MinState,
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

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
    state: State<Warm>,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(state: State<Warm>, high: &[f64], low: &[f64], periods: (usize, usize)) -> Self {
        Self {
            high: high[high.len() - periods.1..].to_vec(),
            low: low[low.len() - periods.1..].to_vec(),
            state,
            periods,
        }
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
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
        cycle(
            (&self.high, &self.low),
            self.periods,
            (&mut lower_line, &mut middle_line, &mut upper_line),
            &mut self.state,
        );

        self.high.drain(..self.high.len() - self.periods.1);
        self.low.drain(..self.low.len() - self.periods.1);

        Ok(vec![lower_line, middle_line, upper_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
}

impl State {
    pub fn new(high: &[f64], low: &[f64], periods: (usize, usize)) -> Self {
        let min_state = MinState::new(low[0], periods.1);
        let max_state = MaxState::new(high[0], periods.1);
        State {
            min_state,
            max_state,
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], look_back: usize) -> State<Warm> {
        let min_state = MinState::init_state(low, look_back);
        let max_state = MaxState::init_state(high, look_back);
        State {
            min_state,
            max_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], usize, (usize, usize));
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, i, periods): Self::Inputs<'a>) -> (f64, f64, f64) {
        let (min, _) = self.min_state.calc((low, i, periods));
        let (max, _) = self.max_state.calc((high, i, periods));

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
        (high, low, i, periods): (&[f64], &[f64], usize, (usize, usize)),
    ) -> (f64, f64, f64) {
        let (min, _) = self.min_state.calc_chuncked_unchecked::<N>((low, i, periods));
        let (max, _) = self.max_state.calc_chuncked_unchecked::<N>((high, i, periods));

        let middle = calc_medprice(max, min);

        (min, middle, max)
    }
}
/// Performs the main calculation loop for the Donchian Channel indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low)` price slices.
/// * `periods` - A tuple of `(period, trail)` where `trail = period - 1`.
/// * `output_lines` - A tuple of mutable slices for storing `(lower, middle, upper)`.
/// * `state` - A mutable reference to the current indicator state.
fn cycle(
    (high, low): (&[f64], &[f64]),
    periods: (usize, usize),
    output_lines: (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State<Warm>,
) {
    let (lower_line, middle_line, upper_line) = output_lines;

    for (j, i) in (periods.1..high.len()).enumerate() {
        unsafe {
            let (lower, middle, upper) = state.calc_unchecked((high, low, i, periods));
            *lower_line.get_unchecked_mut(j) = lower;
            *middle_line.get_unchecked_mut(j) = middle;
            *upper_line.get_unchecked_mut(j) = upper;
        }
    }
}

pub struct DonchianChannel;

impl Indicator<INPUTS, OPTIONS> for DonchianChannel {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
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

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Max::min_data(options))?;

        let periods = (options[0] as usize, options[0] as usize - 1);
        let [high, low] = inputs;

        let (mut lower_line, mut middle_line, mut upper_line) = {
            let capacity = Self::output_length(high.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let mut state = State::init_state(high, low, periods.1);
        cycle(
            (high, low),
            periods,
            (&mut lower_line, &mut middle_line, &mut upper_line),
            &mut state,
        );
        
        Ok((
            vec![lower_line, middle_line, upper_line],
            IndicatorState::new(state, high, low, periods),
        ))
    }
}
