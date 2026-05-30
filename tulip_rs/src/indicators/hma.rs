use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::wma::{calc as calc_wma, multiplier as wma_multiplier, State as WMAState};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

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
/// Returns information about the Hull Moving Average (HMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the HMA indicator.
pub const INFO: Info = Info {
    name: "hma",
    indicator_type: IndicatorType::Trend,
    full_name: "Hull Moving Average",
    inputs: &["real"],
    options: &["period"],
    outputs: &["hma"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "hma",
        label: "HMA",
        display_type: DisplayType::Overlay,
        outputs: &["hma"],
    }],
};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
    real: Vec<f64>,
    period: usize,
    period2: usize,
}
impl IndicatorState {
    pub fn new(
        real: &[f64],
        state: State,
        period: usize,
        period2: usize,
        multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
    ) -> Self {
        Self {
            state,
            multipliers,
            period,
            period2,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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
            self.multipliers,
            self.period,
            &mut hma_line,
        );
        self.real.drain(..self.real.len() - self.period);
        Ok(vec![hma_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub prev_diff: Buffer,
    pub state1: WMAState,
    pub state2: WMAState,
    pub weighted_sumsqrt: f64,
    pub sumsqrt: f64,
}
impl State {
    pub fn new(
        state1: WMAState,
        state2: WMAState,
        weighted_sumsqrt: f64,
        sumsqrt: f64,
        capacity: usize,
    ) -> Self {
        State {
            state1,
            state2,
            weighted_sumsqrt,
            sumsqrt,
            prev_diff: Buffer::new(capacity),
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> (usize, Self) {
        let mut sum: f64 = 0.0;
        let period2 = period / 2;
        //let periodsqrt = (period as f64).sqrt() as usize;
        let mut weighted_sum: f64 = 0.0;
        let mut sum2 = 0.0;
        let mut weighted_sum2 = 0.0;

        for (i, &value) in real.iter().take(period).enumerate() {
            sum += value;
            weighted_sum += value * (i as f64 + 1.0);
            if i >= period - period2 {
                weighted_sum2 += value * (i as f64 + 1.0 - (period - period2) as f64);
                sum2 += value;
            }
        }
        let mut state = Self {
            state1: WMAState::new(sum, weighted_sum),
            state2: WMAState::new(sum2, weighted_sum2),
            weighted_sumsqrt: 0.0,
            sumsqrt: 0.0,
            prev_diff: Buffer::new((period as f64).sqrt() as usize),
        };
        let period2 = period / 2;
        let multiplier = multiplier(period);
        let mut i = period;
        while !state.prev_diff.is_full() {
            calc(
                &mut state,
                (&real[i - period], &real[i - period2]),
                &real[i],
                multiplier,
            );
            i += 1;
        }
        (i, state)
    }
}
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
/// Returns the minimum amount of data required for the HMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the HMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    let period = options[0] as usize;
    let psqrt = (period as f64).sqrt() as usize;
    period + psqrt + 1
}

/// Returns the number of output values produced by the HMA indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the HMA calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Hull Moving Average (HMA) indicator for an entire dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Outputs
///
/// * `outputs[0]` — `hma` line
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; HMA has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `hma` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let period2 = period / 2;
    let multipliers = multiplier(period);

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];
    let mut hma_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let (start, mut state) = State::init_state(real, period);
    cycle_hma(
        real,
        &mut state,
        (period, period2),
        multipliers,
        start,
        &mut hma_line,
    );

    Ok((
        vec![hma_line],
        IndicatorState::new(real, state, period, period2, multipliers),
    ))
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
    state: &mut State,
    periods: (usize, usize),
    multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
    start: usize,
    hma_line: &mut [f64],
) {
    let (period, period2) = periods;

    for (j, i) in (start..real.len()).enumerate() {
        unsafe {
            let (value, prev_values) = (
                real.get_unchecked(i),
                (
                    real.get_unchecked(i - period),
                    real.get_unchecked(i - period2),
                ),
            );
            *hma_line.get_unchecked_mut(j) = calc_unchecked(state, prev_values, value, multipliers);
        }
    }
}

/// Calculates the Hull Moving Average (HMA) for the current data point.
///
/// # Arguments
///
/// * `state` - A mutable reference to the indicator state.
/// * `prev_values` - A tuple of references to the previous values needed for rolling WMA sums:
///   `(prev_value_at_period, prev_value_at_half_period)`.
/// * `value` - A reference to the current input value.
/// * `multipliers` - The precomputed WMA multiplier tuple for both periods.
///
/// # Returns
///
/// The calculated HMA value.
#[inline]
pub fn calc(
    state: &mut State,
    prev_values: (&f64, &f64),
    value: &f64,
    multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
) -> f64 {
    let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
    let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);
    let (prev_value, prev_value2) = prev_values;

    let (wma, _) = calc_wma(&mut state.state1, prev_value, value, multiplier);

    let (wma2, _) = calc_wma(&mut state.state2, prev_value2, value, multiplier2);

    let diff = 2.0 * wma2 - wma;
    weighted_sumsqrt += diff * periodsqrt;
    sumsqrt += diff;

    let prev_diff = &mut state.prev_diff;
    prev_diff.push(diff);

    let mut hma = 0.0;
    if prev_diff.is_full() {
        hma = weighted_sumsqrt / weightssqrt;
        weighted_sumsqrt -= sumsqrt;
        sumsqrt -= unsafe { prev_diff.front_unchecked() };
    } else {
        weighted_sumsqrt -= sumsqrt;
    }
    (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);
    hma
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked(
    state: &mut State,
    prev_values: (&f64, &f64),
    value: &f64,
    multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
) -> f64 {
    let (periodsqrt, weightssqrt, multiplier, multiplier2) = multipliers;
    let (mut weighted_sumsqrt, mut sumsqrt) = (state.weighted_sumsqrt, state.sumsqrt);
    let (prev_value, prev_value2) = prev_values;

    let (wma, _) = calc_wma(&mut state.state1, prev_value, value, multiplier);

    let (wma2, _) = calc_wma(&mut state.state2, prev_value2, value, multiplier2);

    let diff = 2.0 * wma2 - wma;
    weighted_sumsqrt += diff * periodsqrt;
    sumsqrt += diff;

    let prev_diff = &mut state.prev_diff;
    prev_diff.push_unchecked(diff);

    let hma = weighted_sumsqrt / weightssqrt;
    weighted_sumsqrt -= sumsqrt;
    sumsqrt -= prev_diff.front_unchecked();
    (state.weighted_sumsqrt, state.sumsqrt) = (weighted_sumsqrt, sumsqrt);

    hma
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
