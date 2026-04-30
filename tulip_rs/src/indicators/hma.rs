use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::wma::{calc as calc_wma, multiplier as wma_multiplier, State as WMAState};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::hma_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::hma_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::hma_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::hma_simd::indicator_by_options as indicator;
}
/// Returns information about the Hull Moving Average (HMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the HMA indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "hma",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Hull Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["hma"],
        optional_outputs: &[],
    }
}

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

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the HMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Hull Moving Average (HMA) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the HMA calculation.
///
/// # Returns
///
/// A vector of vectors containing the HMA line.

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

/// Performs the main calculation loop for the HMA indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the HMA calculation.
/// * `start` - The starting index for the calculation.
/// * `hma_line` - A mutable reference to a vector for storing the HMA line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
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

/// Calculates the Hull Moving Average (HMA) for the current data point using rolling sums.
///
/// # Arguments
///
/// * `sum` - The rolling sum of the input data.
/// * `weighted_sum` - The rolling weighted sum of the input data.
/// * `prev_value` - The previous value in the input data.
/// * `value` - The new value in the input data.
/// * `period` - The period for the HMA calculation.
///
/// # Returns
///
/// The calculated HMA, WMA, and SMA values.
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

/// Initializes the sums for the HMA calculation.
///
/// # Arguments
///
/// * `prev_real` - A reference to a [f64] containing the previous input values.
///
/// # Returns
///
/// A tuple containing the initial sum and weighted sum.

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
