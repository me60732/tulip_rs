use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::State as MaxState;
use crate::indicators::min::State as MinState;
pub use crate::indicators::rsi::multiplier;
use crate::indicators::rsi::{
    output_length as rsi_output_length, State as RsiState,
};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::ring_buffer::single_buffer::mirror_buffer::{MinMaxBuffer, MirrorBuffer};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::stochrsi_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    period: usize,
    multipliers: (f64, f64),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, period: usize, multipliers: (f64, f64)) -> Self {
        Self {
            period,
            state,
            multipliers,
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
                    self.multipliers,
                    self.period,
                    &mut stochrsi_line,
                    &mut self.state,
                    &mut rsi_line,
                );
            }
            13..30 => {
                cycle_stochrsi::<4>(
                    real,
                    self.multipliers,
                    self.period,
                    &mut stochrsi_line,
                    &mut self.state,
                    &mut rsi_line,
                );
            }
            _ => {
                cycle_stochrsi::<8>(
                    real,
                    self.multipliers,
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
            rsi = rsi_state.calc(real[i], multiplier);
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
pub const INFO: Info = Info {
    name: "stochrsi",
    full_name: "Stochastic RSI",
    indicator_type: IndicatorType::Momentum,
    inputs: &["real"],
    options: &["period"],
    outputs: &["stochrsi"],
    optional_outputs: &["rsi"],
    display_groups: &[DisplayGroup {
        id: "stochrsi",
        label: "STOCHRSI",
        display_type: DisplayType::Indicator,
        outputs: &["stochrsi", "rsi"],
    }],
};
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the smoothing
/// multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Info(INFO),
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

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Stochastic RSI calculation.
///
/// # Returns
///
/// The output length for the Stochastic RSI calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Stochastic RSI indicator values over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (price series)
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice controlling extra output series;
///   `optional_outputs[0] = true` enables the `rsi` output.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `stochrsi`,
/// `outputs[1]` is `rsi` (empty unless requested), and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let capacity = output_length(real.len(), options);
    let rsi_capacity = rsi_output_length(real.len(), options);
    let mut stochrsi_line = crate::uninit_vec!(f64, capacity); //vec![0.0; capacity]; // Vec::with_capacity(capacity);
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
                multipliers,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
        6..30 => {
            cycle_stochrsi::<4>(
                real,
                multipliers,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
        _ => {
            cycle_stochrsi::<8>(
                real,
                multipliers,
                period,
                &mut stochrsi_line,
                &mut state,
                rsi,
            );
        }
    }

    Ok((
        vec![stochrsi_line, rsi_line],
        IndicatorState::new(state, period, multipliers),
    ))
}

/// Performs the main calculation loop for the Stochastic RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `multiplier` - The EMA multiplier derived from the period.
/// * `period` - The period for the Stochastic RSI calculation.
/// * `stochrsi_line` - A mutable slice for storing the Stochastic RSI output values.
/// * `state` - A mutable reference to the current indicator state.
/// * `rsi_line` - A mutable slice for storing the optional RSI output values.
fn cycle_stochrsi<const N: usize>(
    real: &[f64],
    multipliers: (f64, f64),
    period: usize,
    stochrsi_line: &mut [f64],
    state: &mut State,
    rsi_line: &mut [f64],
) {
    let (_, want_rsi) = crate::calc_want_flags!(rsi_line);

    for i in 0..real.len() {
        let val = unsafe { *real.get_unchecked(i) };

        let (kfast, rsi) = calc::<N>(state, val, multipliers, period);

        unsafe { *stochrsi_line.get_unchecked_mut(i) = kfast };
        crate::store_optional_outputs!(i,
            want_rsi, rsi_line => rsi
        );
    }
}

/// Calculates a single Stochastic RSI value from the current state.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current indicator state.
/// * `real` - The current real price value.
/// * `multiplier` - The EMA multiplier derived from the period.
/// * `period` - The period for the Stochastic RSI calculation.
///
/// # Returns
///
/// A tuple `(kfast, rsi)` where `kfast` is the Stochastic RSI value and `rsi` is the current RSI value.
///
/// # Note on scaling
///
/// This implementation outputs StochRSI on a 0–100 scale, matching the
/// standard Stochastic Oscillator (%K).
///
/// In the original publication — Chande & Kroll, “The New Technical Trader”
/// (1994) — the StochRSI formula was printed without the ×100 scaling
/// factor. This omission was a typesetting error, but it led most
/// indicator libraries to adopt a 0–1 ratio instead.
///
/// Users migrating from libraries that follow the misprinted 0–1 convention
/// should be aware of this difference.
#[inline(always)]
pub fn calc<const N: usize>(
    state: &mut State,
    real: f64,
    multipliers: (f64, f64),
    period: usize,
) -> (f64, f64) {
    let rsi = state.rsi_state.calc(real, multipliers);
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
