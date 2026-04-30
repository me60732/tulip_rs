use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::State as MaxState;
use crate::indicators::min::State as MinState;
pub use crate::indicators::rsi::multiplier;
use crate::indicators::rsi::{
    calc as rsi_calc, output_length as rsi_output_length, State as RsiState,
};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::ring_buffer::single_buffer::mirror_buffer::{MinMaxBuffer, MirrorBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    period: usize,
    multiplier: f64,
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, period: usize, multiplier: f64) -> Self {
        Self {
            period,
            state,
            multiplier
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let capacity = inputs[0].len();
        let mut rsi_line = crate::init_optional_outputs!(
            optional_outputs, &[false],
            rsi_line: capacity
        );

        let real = inputs[0];
        let mut stochrsi_line = vec![0.0; capacity];

        match self.period {
            1..=12 => {
                cycle_stochrsi::<1>(
                    real,
                    self.multiplier,
                    self.period,
                    &mut stochrsi_line,
                    &mut self.state,
                    &mut rsi_line,
                );
            }
            13..30 => {
                cycle_stochrsi::<4>(
                    real,
                    self.multiplier,
                    self.period,
                    &mut stochrsi_line,
                    &mut self.state,
                    &mut rsi_line,
                );
            }
            _ => {
                cycle_stochrsi::<8>(
                    real,
                    self.multiplier,
                    self.period,
                    &mut stochrsi_line,
                    &mut self.state,
                    &mut rsi_line,
                );
            }
        }

        Ok(vec![stochrsi_line, rsi_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub min_state: MinState,
    pub max_state: MaxState,
    pub rsi_state: RsiState,
}
impl State {
    pub fn init_state(real: &[f64], period: usize, rsi_line: &mut [f64]) -> State {
        let mut rsi_state = RsiState::init_state(real, period);
        let mut buffer = Buffer::new(period);
        let mut rsi = 100.0 * (rsi_state.up_sum / (rsi_state.up_sum + rsi_state.down_sum));
        buffer.push(rsi);
        let mut min_state = MinState::new(rsi, period);
        let mut max_state = MaxState::new(rsi, period);
        let multiplier = multiplier(period);
        let mut i = period + 1;
        while buffer.get_count() < buffer.get_capacity() {
            rsi = rsi_calc(&mut rsi_state, real[i], multiplier);
            buffer.push(rsi);
            buffer.min::<1>(&mut min_state, rsi, period);
            buffer.max::<1>(&mut max_state, rsi, period);
            crate::init_store_optional_outputs!(i, real.len(), rsi_line => rsi);
            i += 1;
        }
        State {
            min_state,
            max_state,
            rsi_state,
            buffer,
        }
    }
}
/// Returns information about the Stochastic RSI indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Stochastic RSI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "stochrsi",
        full_name: "Stochastic RSI",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["stochrsi"],
        optional_outputs: &["rsi"],
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize)],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the Stochastic RSI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Stochastic RSI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    (options[0]) as usize * 2 + 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Stochastic RSI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length for the Stochastic RSI calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Stochastic RSI indicator values.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (real prices).
/// * `options` - A slice containing the options for the Stochastic RSI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the Stochastic RSI indicator values and the state.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let capacity = output_length(real.len(), options);
    let rsi_capacity = rsi_output_length(real.len(), options);
    let mut stochrsi_line = crate::uninit_vec!(f64, capacity);//vec![0.0; capacity]; // Vec::with_capacity(capacity);
    let mut rsi_line = crate::init_optional_outputs_eff!(
        optional_outputs, &[false],
        rsi_line: rsi_capacity
    );
    let mut state = State::init_state(real, period, &mut rsi_line);
    let rsi = {
        let offset = crate::slice_outputs_start!(stochrsi_line.len(), rsi_line);
        &mut rsi_line[offset..]
    };
    let real = &real[period * 2..];

    match period {
        1..=5 => {
            cycle_stochrsi::<1>(
                real,
                multiplier,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
        6..30 => {
            cycle_stochrsi::<4>(
                real,
                multiplier,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
        _ => {
            cycle_stochrsi::<8>(
                real,
                multiplier,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
    }

    Ok((
        vec![stochrsi_line, rsi_line],
        IndicatorState::new(state, period, multiplier),
    ))
}

/// Calculates the Stochastic RSI indicator values from the previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data (real prices).
/// * `options` - A slice containing the options for the Stochastic RSI calculation.
/// * `indicator_state` - The previous state of the Stochastic RSI indicator.
/// * `optional_outputs` - An optional slice indicating whether to calculate optional outputs.
///
/// # Returns
///
/// An `Output` struct containing the Stochastic RSI indicator values and the updated state.

/// Performs the main calculation loop for the Stochastic RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `rsi_period` - The period for the RSI calculation.
/// * `stoch_period` - The period for the Stochastic RSI calculation.
/// * `stochrsi_line` - A mutable reference to a vector for storing the Stochastic RSI line.
/// * `min_max` - A tuple containing the minimum value, trailing index for the minimum value, maximum value, and trailing index for the maximum value.
/// * `sums` - A tuple containing the sums of up and down values.
///
/// # Returns
///
/// A tuple containing the updated min, max, trail_min, trail_max, up_sum, and down_sum values.
fn cycle_stochrsi<const N: usize>(
    real: &[f64],
    multiplier: f64,
    period: usize,
    stochrsi_line: &mut [f64],
    state: &mut State,
    rsi_line: &mut [f64],
) {

    let (_, want_rsi) = crate::calc_want_flags!(rsi_line);

    for i in 0..real.len() {
        let val = unsafe { *real.get_unchecked(i) };

        let (kfast, rsi) = calc::<N>(state, val, multiplier, period);

        unsafe { *stochrsi_line.get_unchecked_mut(i) = kfast };
        crate::store_optional_outputs!(i,
            want_rsi, rsi_line => rsi
        );
    }
}

#[inline(always)]
pub fn calc<const N: usize>(
    state: &mut State,
    real: f64,
    multiplier: f64,
    period: usize,
) -> (f64, f64) {
    let rsi = rsi_calc(&mut state.rsi_state, real, multiplier);
    state.buffer.push(rsi);

    let (min, _) = state.buffer.min::<N>(&mut state.min_state, rsi, period);
    let (max, _) = state.buffer.max::<N>(&mut state.max_state, rsi, period);

    let kdif = max - min;
    let kfast = if kdif < f64::EPSILON {
        0.0
    } else {
        100.0 * (rsi - min) / kdif
    };

    (kfast, rsi)
}
