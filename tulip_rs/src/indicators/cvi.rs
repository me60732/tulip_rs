use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::calc as calc_ema;
pub use crate::indicators::ema::multiplier;
pub use crate::ring_buffer::single_buffer::generic_buffer::{Buffer as State, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::cvi_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::cvi_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::cvi_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::cvi_simd::indicator_by_options as indicator;
}

pub trait BufferExt {
    fn init_state(inputs: &[&[f64]; INPUTS_WIDTH], period: usize) -> State;
}
impl BufferExt for State {
    fn init_state(inputs: &[&[f64]; INPUTS_WIDTH], period: usize) -> Self {
        let mut prev_ema = State::new(period);

        let (high, low) = (inputs[0], inputs[1]);
        let multiplier = multiplier(period);
        for (i, (&h, &l)) in high.iter().zip(low.iter()).enumerate().take(period * 2 - 1) {
            if i < period {
                let hl_diff = (h - l).max(f64::EPSILON);
                let base = prev_ema.back().unwrap_or(hl_diff);

                let ema = calc_ema(&hl_diff, base, multiplier);
                prev_ema.push(ema);
                continue;
            }
            calc(&mut prev_ema, &h, &l, multiplier);
        }

        prev_ema
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multiplier: (f64, f64)) -> Self {
        Self { state, multiplier }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut cvi_line = crate::uninit_vec!(f64, inputs[0].len());
        let [high, low] = inputs;
        cycle(high, low, self.multiplier, &mut self.state, &mut cvi_line);

        Ok(vec![cvi_line])
    }
}
/// Returns information about the Chaikin Volatility Indicator (CVI).
///
/// # Returns
///
/// An `Info` struct containing metadata about the CVI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "cvi",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Chaikin Volatility Indicator",
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["cvi"],
        optional_outputs: &[],
    }
}
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
        IndicatorInfoOrInteger::Integer(1),
        min_data,
    )
}
/// Returns the minimum amount of data required for the CVI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the CVI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    (options[0] * 2.0) as usize
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the CVI calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Chaikin Volatility Indicator (CVI) over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period (EMA window used to smooth the high-low range)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; CVI has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `cvi` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    let mut cvi_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(inputs, period);

    let multiplier = multiplier(period);
    let (high, low) = {
        let from = period * 2 - 1;
        (&inputs[0][from..], &inputs[1][from..])
    };
    cycle(high, low, multiplier, &mut state, &mut cvi_line);

    Ok((vec![cvi_line], IndicatorState { state, multiplier }))
}

/// Performs the main calculation loop for the CVI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `multiplier` - A tuple `(multiplier, inv_multiplier)` derived from the EMA period.
/// * `state` - Mutable reference to the ring buffer holding recent EMA values.
/// * `cvi_line` - Mutable slice to write the CVI output values into.
fn cycle(
    high: &[f64],
    low: &[f64],
    multiplier: (f64, f64),
    state: &mut State,
    cvi_line: &mut [f64],
) {
    for i in 0..high.len() {
        unsafe {
            *cvi_line.get_unchecked_mut(i) = calc_unchecked(
                state,
                high.get_unchecked(i),
                low.get_unchecked(i),
                multiplier,
            );
        }
    }
}

/// Calculates the current CVI value.
///
/// # Arguments
///
/// * `buffer` - Mutable reference to the ring buffer holding recent EMA values.
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `multiplier` - A tuple `(multiplier, inv_multiplier)` for the EMA calculation.
///
/// # Returns
///
/// The CVI value as a percentage change between the current and oldest EMA in the buffer.
#[inline]
pub fn calc(buffer: &mut State, high: &f64, low: &f64, multiplier: (f64, f64)) -> f64 {
    let prev_ema = buffer.back().unwrap();
    let old_ema = buffer.front().unwrap();
    let hl_diff = (high - low).max(f64::EPSILON);
    let ema = calc_ema(&hl_diff, prev_ema, multiplier);
    buffer.push(ema);
    if old_ema.abs() < f64::EPSILON {
        0.0
    } else {
        (ema - old_ema) / old_ema * 100.0
    }
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked(
    buffer: &mut State,
    high: &f64,
    low: &f64,
    multiplier: (f64, f64),
) -> f64 {
    let prev_ema = buffer.back_unchecked();
    let old_ema = buffer.front_unchecked();
    let hl_diff = (high - low).max(f64::EPSILON);
    let ema = calc_ema(&hl_diff, prev_ema, multiplier);
    buffer.push_unchecked(ema);

    (ema - old_ema) / old_ema * 100.0
}
