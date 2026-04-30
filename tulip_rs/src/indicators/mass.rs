use crate::common::{min_process, validate_inputs, validate_options};
use crate::indicators::ema::calc as calc_ema;
use crate::indicators::ema::multiplier as ema_multiplier;

pub use crate::indicator_types::TIndicatorState;
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::mass_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::mass_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::mass_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::mass_simd::indicator_by_options as indicator;
}

/// Returns information about the Mass Index (Mass) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Mass indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "mass",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        full_name: "Mass Index",
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["mass"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64)) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut mass_line = crate::uninit_vec!(f64, inputs[0].len());
        let [high, low] = inputs;
        cycle_mass(
            high,
            low,
            self.multipliers,
            &mut mass_line,
            &mut self.state,
        );

        Ok(vec![mass_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub sum: f64,
    pub ema: f64,
    pub ema_signal: f64,
}
impl State {
    /*pub fn new(ema: f64, ema_signal: f64, period: usize) -> Self {
        Self {
            ema,
            sum,
            ema_signal,
            buffer: Buffer::new(period),
        }
    }*/
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        multiplier: (f64, f64),
        mass_line: &mut [f64],
    ) -> (usize, State) {
        let (mut ema, mut ema_signal, mut buffer, mut sum) =
            (high[0] - low[0], 0.0, Buffer::new(period), 0.0);
        let mut i = 1;
        while !buffer.is_full() {
            let hl_diff = high[i] - low[i];
            ema = calc_ema(&hl_diff, ema, multiplier);
            if i == 8 {
                ema_signal = ema;
            }
            if i >= 8 {
                ema_signal = calc_ema(&ema, ema_signal, multiplier);
                if i >= 16 {
                    let mass = (ema / ema_signal).max(0.0);
                    sum += mass;
                    buffer.push(mass);
                    if buffer.is_full() {
                        mass_line[0] = sum;
                    }
                }
            }
            i += 1;
        }
        (
            i,
            State {
                sum,
                ema,
                ema_signal,
                buffer,
            },
        )
    }
    pub fn get_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier().0],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the Mass indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Mass calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 16
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Mass calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Mass Index (Mass) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the Mass calculation.
///
/// # Returns
///
/// A vector of vectors containing the Mass line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let mut mass_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let multipliers = multiplier();
    let (high, low, mut state) = {
        let (start, state) = State::init_state(
            inputs[0],
            inputs[1],
            options[0] as usize,
            multipliers,
            &mut mass_line,
        );
        (&inputs[0][start..], &inputs[1][start..], state)
    };
    

    cycle_mass(
        high,
        low,
        multipliers,
        &mut mass_line[1..],
        &mut state,
    );

    Ok((vec![mass_line], IndicatorState { multipliers, state }))
}

/// Performs the main calculation loop for the Mass indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `period` - The period for the Mass calculation.
/// * `mass_line` - A mutable reference to a vector for storing the Mass line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
/// * `prev_state` - An optional tuple containing the previous state values.
fn cycle_mass(
    high: &[f64],
    low: &[f64],
    multipliers: (f64, f64),
    mass_line: &mut [f64],
    state: &mut State,
) {
    for i in 0..high.len() {
        unsafe {
            *mass_line.get_unchecked_mut(i) = calc_unchecked(
                state,
                high.get_unchecked(i),
                low.get_unchecked(i),
                multipliers,
            );
        }
    }
}

/// Calculates the Mass Index (Mass) for the current data point.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `period` - The period for the Mass calculation.
/// * `ema` - The previous EMA value.
/// * `ema_signal` - The previous EMA signal value.
///
/// # Returns
///
/// A tuple containing the calculated Mass, new EMA, and new EMA signal values.
#[inline(always)]
pub fn calc(state: &mut State, high: &f64, low: &f64, multiplier: (f64, f64)) -> f64 {
    let hl_diff = (high - low).max(f64::EPSILON);
    let mut ema = state.ema;
    let mut ema_signal = state.ema_signal;
    ema = calc_ema(&hl_diff, ema, multiplier);
    ema_signal = calc_ema(&ema, ema_signal, multiplier);
    let mass = (ema / ema_signal).max(0.0);
    if let Some(old) = state.buffer.push_with_info(mass) {
        state.sum -= old
    }
    state.sum += mass;

    (state.ema, state.ema_signal) = (ema, ema_signal);
    state.sum
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked(
    state: &mut State,
    high: &f64,
    low: &f64,
    multiplier: (f64, f64),
) -> f64 {
    let hl_diff = (high - low).max(f64::EPSILON);
    let (mut ema, mut ema_signal) = (state.ema, state.ema_signal);
    ema = calc_ema(&hl_diff, ema, multiplier);
    ema_signal = calc_ema(&ema, ema_signal, multiplier);
    let mass = (ema / ema_signal).max(0.0);
    state.sum += mass - state.buffer.push_with_info_unchecked(mass);

    (state.ema, state.ema_signal) = (ema, ema_signal);
    state.sum
}

pub fn multiplier() -> (f64, f64) {
    ema_multiplier(9)
}
