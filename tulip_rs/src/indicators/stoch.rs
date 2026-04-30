use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::{calc as calc_max, calc_unchecked as calc_max_unchecked};
use crate::indicators::min::{calc as calc_min, calc_unchecked as calc_min_unchecked};
pub use crate::indicators::{max::State as MaxState, min::State as MinState};

use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 3;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::stoch_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::stoch_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::stoch_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::stoch_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    high: Vec<f64>,
    low: Vec<f64>,
    multipliers: (f64, f64),
    k_period: usize,
}
impl IndicatorState {
    pub fn new(state: State, high: &[f64], low: &[f64], multipliers: (f64, f64), k_period: usize) -> Self {
        Self {
            state,
            multipliers,
            high: high[high.len() - k_period..].to_vec(),
            low: low[low.len() - k_period..].to_vec(),
            k_period
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

        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let close = inputs[2];

        let (mut k_line, mut d_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        match self.k_period {
            1..=4 => {
                cycle::<1>(
                    (&self.high, &self.low, close),
                    self.k_period, 
                    0,
                    self.multipliers,
                    &mut self.state,
                    (&mut k_line, &mut d_line),
                );
            }
            5..30 => {
                cycle::<4>(
                    (&self.high, &self.low, close),
                    self.k_period, 
                    0,
                    self.multipliers,
                    &mut self.state,
                    (&mut k_line, &mut d_line),
                );
            }
            _ => {
                cycle::<8>(
                    (&self.high, &self.low, close),
                    self.k_period, 
                    0,
                    self.multipliers,
                    &mut self.state,
                    (&mut k_line, &mut d_line),
                );
            }
        }

        self.high.drain(..self.high.len() - self.k_period);
        self.low.drain(..self.low.len() - self.k_period);

        Ok(vec![k_line, d_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub prev_k: Buffer,
    pub prev_d: Buffer,
    pub min_state: MinState,
    pub max_state: MaxState,
    pub k_sum: f64,
    pub d_sum: f64,
}

impl State {
    pub fn new(min: (f64, usize), max: (f64, usize), k_slow: usize, d_period: usize) -> Self {
        State {
            min_state: MinState::new(min.0, min.1),
            max_state: MaxState::new(max.0, max.1),
            prev_k: Buffer::new(k_slow),
            prev_d: Buffer::new(d_period),
            k_sum: 0.0,
            d_sum: 0.0,
        }
    }
    pub fn init_state(
        inputs: (&[f64], &[f64], &[f64]),
        k_period: usize,
        k_slow: usize,
        d_period: usize,
        k_line: &mut [f64],
    ) -> (Self, usize, usize) {
        let (high, low, _) = inputs;
        let mut state = Self::new((low[0], k_period), (high[0], k_period), k_slow, d_period);
        let (k_multiplier, _d_multiplier) = &multiplier(k_slow, d_period);
        let mut k_count = 0;
        let mut start = 0;
        for i in k_period + 1..k_period + k_slow + d_period {
            let k_fast = calc_kfast(
                &mut state.min_state,
                &mut state.max_state,
                inputs,
                i,
                k_period,
            );
            state.k_sum += k_fast;
            if let Some(k_old) = state.prev_k.push_with_info(k_fast) {
                state.k_sum -= k_old;
            }
            if state.prev_k.is_full() {
                // Buffer was full so a value was replaced.
                let k = state.k_sum * k_multiplier;
                k_line[k_count] = k;
                k_count += 1;
                state.d_sum += k;
                state.prev_d.push(k);
            }
            start = i;
        }
        start += 1;
        (state, k_count, start)
    }
}
/// Returns information about the Stochastic Oscillator indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Stochastic Oscillator indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "stoch",
        full_name: "Stochastic Oscillator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        inputs: &["high", "low", "close"],
        options: &["k_period", "k_slow", "d_period"],
        outputs: &["stoch_k", "stoch_d"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Stochastic Oscillator indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Stochastic Oscillator calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    (options[0] + options[1] + options[2]) as usize + 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Stochastic Oscillator calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length for the Stochastic Oscillator calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> (usize, usize) {
    let d_capacity = data_len - min_data(options) + 1;
    (d_capacity + options[2] as usize, d_capacity)
}

/// Calculates the Stochastic Oscillator indicator values.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (high, low, and close prices).
/// * `options` - A slice containing the options for the Stochastic Oscillator calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the Stochastic Oscillator indicator values and the state.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let k_period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let [high, low, close] = inputs;

    let (mut k_line, mut d_line, mut state, outputs, start, multipliers);
    {
        let (k_capacity, d_capacity) = output_length(high.len(), options);
        k_line = crate::uninit_vec!(f64, k_capacity);
        d_line = crate::uninit_vec!(f64, d_capacity);

        let k_slow = options[1] as usize;
        let d_period = options[2] as usize;
        multipliers = multiplier(k_slow, d_period);
        let k_count;
        (state, k_count, start) =
            State::init_state((high, low, close), k_period, k_slow, d_period, &mut k_line);
        outputs = (&mut k_line[k_count..], d_line.as_mut_slice());
    }
    //println!("k_line: {:?}, d_line: {:?}, start: {:?}, k_count: {:?}", k_line.len(), d_line.len(), start, k_count);
    match k_period {
        1..=4 => {
            cycle::<1>(
                (high, low, close),
                k_period, 
                start,
                multipliers,
                &mut state,
                outputs,
            );
        }
        5..30 => {
            cycle::<4>(
                (high, low, close),
                k_period, 
                start,
                multipliers,
                &mut state,
                outputs,
            );
        }
        _ => {
            cycle::<8>(
                (high, low, close),
                k_period, 
                start,
                multipliers,
                &mut state,
                outputs,
            );
        }
    }

    Ok((
        vec![k_line, d_line],
        IndicatorState::new(state, high, low, multipliers, k_period)
    ))
}

/// Calculates the Stochastic Oscillator indicator values from the previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (high, low, and close prices).
/// * `options` - A slice containing the options for the Stochastic Oscillator calculation.
/// * `indicator_state` - The previous state of the Stochastic Oscillator indicator.
/// * `optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the Stochastic Oscillator indicator values and the updated state.

/// Performs the main calculation loop for the Stochastic Oscillator indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `period` - The period for the Stochastic Oscillator calculation.
/// * `k_line` - A mutable reference to a vector for storing the %K line.
/// * `d_line` - A mutable reference to a vector for storing the %D line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
/// * `min` - The minimum value.
/// * `max` - The maximum value.
/// * `trail_min` - The trailing index for the minimum value.
/// * `trail_max` - The trailing index for the maximum value.
///
/// # Returns
///
/// A tuple containing the updated min, max, trail_min, and trail_max values.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64], &[f64]),
    k_period: usize,
    start: usize,
    multipliers: (f64, f64),
    state: &mut State,
    outputs: (&mut [f64], &mut [f64]),
) {
    let close = inputs.2;
    let (k_line, d_line) = outputs;

    for (j, i) in (start..close.len()).enumerate() {
        unsafe {
            (
                *k_line.get_unchecked_mut(j),
                *d_line.get_unchecked_mut(j),
            ) = calc_unchecked::<N>(state, inputs, i, k_period, multipliers);
        }
        //k_count += 1;
    }
}
/// Calculates the Stochastic Oscillator values for a single data point.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `i` - The current index.
/// * `min` - The minimum value.
/// * `trail_min` - The trailing index for the minimum value.
/// * `max` - The maximum value.
/// * `trail_max` - The trailing index for the maximum value.
/// * `period` - The period for the Stochastic Oscillator calculation.
///
/// # Returns
///
/// A tuple containing the %K, %D, min, max, trail_min, and trail_max values.
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64], &[f64]),
    i: usize,
    k_period: usize,
    multipliers: (f64, f64),
) -> (f64, f64) {
    let (k_multiplier, d_multiplier) = multipliers;

    let kfast = calc_kfast(
        &mut state.min_state,
        &mut state.max_state,
        inputs,
        i,
        k_period,
    );

    if let Some(old_k) = state.prev_k.push_with_info(kfast) {
        state.k_sum += kfast - old_k;
    } else {
        state.k_sum += kfast;
    }
    let k = state.k_sum * k_multiplier;
    if let Some(old_d) = state.prev_d.push_with_info(k) {
        state.d_sum += k - old_d;
    } else {
        state.d_sum += k;
    }

    (k, state.d_sum * d_multiplier)
}
#[inline(always)]
unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    inputs: (&[f64], &[f64], &[f64]),
    i: usize,
    k_period: usize,
    multipliers: (f64, f64),
) -> (f64, f64) {
    let (k_multiplier, d_multiplier) = multipliers;

    let kfast = calc_kfast_unchecked::<N>(
        &mut state.min_state,
        &mut state.max_state,
        inputs,
        i,
        k_period,
    );

    let old_k = state.prev_k.push_with_info_unchecked(kfast);
    state.k_sum += kfast - old_k;
    let k = state.k_sum * k_multiplier;
    let old_d = state.prev_d.push_with_info_unchecked(k);
    state.d_sum += k - old_d;

    (k, state.d_sum * d_multiplier)
}

#[inline(always)]
pub fn calc_kfast(
    min_state: &mut MinState,
    max_state: &mut MaxState,
    inputs: (&[f64], &[f64], &[f64]),
    i: usize,
    period: usize,
) -> f64 {
    let (high, low, close) = inputs;
    let shift = low.len() - close.len();

    let (min, _) = calc_min(min_state, low, i + shift, (period, period - 1));
    let (max, _) = calc_max(max_state, high, i + shift, (period, period - 1));

    100.0 * (close[i] - min) / (max - min).max(f64::EPSILON)
}
#[inline(always)]
pub unsafe fn calc_kfast_unchecked<const N: usize>(
    min_state: &mut MinState,
    max_state: &mut MaxState,
    inputs: (&[f64], &[f64], &[f64]),
    i: usize,
    period: usize,
) -> f64 {
    let (high, low, close) = inputs;
    let shift = low.len() - close.len();

    let (min, _) = calc_min_unchecked::<N>(min_state, low, i + shift, (period, period - 1));
    let (max, _) = calc_max_unchecked::<N>(max_state, high, i + shift, (period, period - 1));

    100.0 * (close.get_unchecked(i) - min) / (max - min).max(f64::EPSILON)
}

#[inline(always)]
pub fn multiplier(k_slow: usize, d_period: usize) -> (f64, f64) {
    (1.0 / k_slow as f64, 1.0 / d_period as f64)
}
