use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::wma::{multiplier as wma_multiplier, State as WMAState};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::hma_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::hma_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::hma_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::hma_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    real: Vec<f64>,
    period: usize,
    period2: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize, period2: usize) -> Self {
        Self {
            state,
            period,
            period2,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut hma_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        cycle_hma(
            &self.real,
            &mut self.state,
            (self.period, self.period2),
            self.period,
            &mut hma_line,
        );
        self.real.drain(..self.real.len() - self.period);
        Ok(vec![hma_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub prev_diff: Buffer<S>,
    pub state1: WMAState<S>,
    pub state2: WMAState<S>,
    pub weighted_sumsqrt: f64,
    pub(crate) weightssqrt: f64,
    pub(crate) periodsqrt: f64,
    pub sumsqrt: f64,
}
impl State<Cold> {
    pub fn new(
        state1: WMAState,
        state2: WMAState,
        weighted_sumsqrt: f64,
        sumsqrt: f64,
        multipliers: (f64, f64),
        capacity: usize,
    ) -> State<Cold> {
        State {
            state1,
            state2,
            weighted_sumsqrt,
            sumsqrt,
            periodsqrt: multipliers.0,
            weightssqrt: multipliers.1,
            prev_diff: Buffer::new(capacity),
        }
    }

    pub fn init_state(real: &[f64], period: usize) -> (usize, State<Warm>) {
        let period2 = period / 2;
        let periodsqrt = ((period as f64).sqrt() as usize) as f64;
        let weightssqrt = periodsqrt * (periodsqrt + 1.0) / 2.0;

        let mut state1 = WMAState::init_state(real, period);
        let mut state2 = WMAState::init_state(&real[period - period2..], period2);
        let mut prev_diff = Buffer::new(periodsqrt as usize);
        let mut weighted_sumsqrt = 0.0;
        let mut sumsqrt = 0.0;

        let mut i = period;
        let mut first_diff = 0.0_f64;
        while !prev_diff.is_full() {
            let (wma, _) = state1.calc((real[i], real[i-period]));
            let (wma2, _) = state2.calc((real[i], real[i-period2]));
            let diff = 2.0 * wma2 - wma;
            if i == period { first_diff = diff; }
            weighted_sumsqrt += diff * periodsqrt;
            sumsqrt += diff;
            prev_diff.push(diff);
            weighted_sumsqrt -= sumsqrt;  // ← missing in current code
            i += 1;
        }
        sumsqrt -= first_diff;  // ← missing: the sumsqrt -= front() that happens on fill

        (
            i,
            State {
                state1,
                state2,
                weighted_sumsqrt,
                sumsqrt,
                prev_diff: prev_diff.into_full(),
                periodsqrt,
                weightssqrt,
            },
        )
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value, prev_value2): Self::Inputs<'a>) -> Self::Outputs {
        let (wma, _) = self.state1.calc((value, prev_value));

        let (wma2, _) = self.state2.calc((value, prev_value2));

        let diff = 2.0 * wma2 - wma;
        self.weighted_sumsqrt += diff * self.periodsqrt;
        self.sumsqrt += diff;

        let prev_diff = &mut self.prev_diff;
        prev_diff.push(diff);

        let hma = self.weighted_sumsqrt / self.weightssqrt;
        self.weighted_sumsqrt -= self.sumsqrt;
        self.sumsqrt -= prev_diff.front();

        hma
    }
}

/// Performs the main calculation loop for the HMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the indicator state.
/// * `periods` - A tuple `(period, period2)` for the full and half WMA periods.
/// * `multipliers` - The precomputed WMA multiplier tuple for both periods.
/// * `start` - The starting index within `real` for the calculation.
/// * `hma_line` - A mutable slice for storing the HMA output values.
//#[inline(always)]
fn cycle_hma(
    real: &[f64],
    state: &mut State<Warm>,
    periods: (usize, usize),
    start: usize,
    hma_line: &mut [f64],
) {
    let (period, period2) = periods;
    for (j, i) in (start..real.len()).enumerate() {
        unsafe {
            let inputs = (
                *real.get_unchecked(i),
                *real.get_unchecked(i - period),
                *real.get_unchecked(i - period2),
            );
            *hma_line.get_unchecked_mut(j) = state.calc(inputs);
        }
    }
}

/// Returns the precomputed WMA multipliers for the HMA calculation.
///
/// # Arguments
///
/// * `period` - The HMA period.
///
/// # Returns
///
/// A tuple `(periodsqrt, weightssqrt, multiplier, multiplier2)` where `multiplier` and
/// `multiplier2` are the WMA multiplier tuples for `period` and `period/2` respectively.
pub fn multiplier(period: usize) -> (f64, f64, (f64, f64, f64), (f64, f64, f64)) {
    let periodsqrt = ((period as f64).sqrt() as usize) as f64;
    let weightssqrt = periodsqrt * (periodsqrt + 1.0) / 2.0;
    (
        periodsqrt,
        weightssqrt,
        wma_multiplier(period),
        wma_multiplier(period / 2),
    )
}

pub struct Hma;

impl Indicator<INPUTS, OPTIONS> for Hma {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "hma",
        indicator_type: IndicatorType::Trend,
        full_name: "Hull Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["hma"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "hma",
            label: "HMA",
            display_type: DisplayType::Overlay,
            outputs: &["hma"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        let period = options[0] as usize;
        let psqrt = (period as f64).sqrt() as usize;
        period + psqrt + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;
        let period2 = period / 2;

        validate_inputs(inputs, Self::min_data(options))?;

        let real = inputs[0];
        let mut hma_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let (start, mut state) = State::init_state(real, period);
        cycle_hma(real, &mut state, (period, period2), start, &mut hma_line);

        Ok((
            vec![hma_line],
            IndicatorState::new(real, state, period, period2),
        ))
    }
}
