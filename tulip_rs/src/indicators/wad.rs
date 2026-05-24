use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wad_simd::indicator_by_assets;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::wad_simd::indicator_by_assets as indicator;
}

pub fn info() -> Info<'static> {
    Info {
        name: "wad",
        full_name: "WAD Indicator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["wad"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub prev_close: f64,
    pub wad: f64,
}
impl IndicatorState {
    pub fn new(prev_close: f64, wad: f64) -> Self {
        Self { prev_close, wad }
    }
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64) -> f64 {
        self.wad += if close > self.prev_close {
            close - self.prev_close.min(low)
        } else if close < self.prev_close {
            close - self.prev_close.max(high)
        } else {
            0.0
        };

        self.prev_close = close;

        self.wad
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut wad_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs[0], inputs[1], inputs[2], self, &mut wad_line);

        Ok(vec![wad_line])
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (unused; WAD takes no options).
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the WAD indicator.
///
/// # Arguments
///
/// * `_options` - Unused; WAD takes no options.
///
/// # Returns
///
/// The minimum amount of data required (2: one bar to seed the previous close,
/// one bar to produce the first output).
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the WAD calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Williams Accumulation/Distribution (WAD) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `high`
/// * `inputs[1]` — `low`
/// * `inputs[2]` — `close`
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `_options` - Unused; WAD takes no options.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `wad` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    // Expecting three inputs: High, Low, Close.
    validate_inputs(inputs, min_data(_options))?;

    let mut wad_line: Vec<f64> = {
        let capacity = output_length(inputs[0].len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = IndicatorState::new(inputs[2][0], 0.0);

    cycle(
        &inputs[0][1..],
        &inputs[1][1..],
        &inputs[2][1..],
        &mut state,
        &mut wad_line,
    );

    // Store last used close and sum for incremental updates.
    Ok((vec![wad_line], state))
}

/// Iterates over the high, low, and close slices and computes WAD values for each bar.
///
/// # Arguments
///
/// * `high` - Input high price slice.
/// * `low` - Input low price slice.
/// * `close` - Input close price slice.
/// * `state` - Mutable reference to the `IndicatorState` (previous close and cumulative WAD).
/// * `wad_line` - Mutable output slice for WAD values.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut IndicatorState,
    wad_line: &mut [f64],
) {
    for i in 0..close.len() {
        unsafe {
            *wad_line.get_unchecked_mut(i) = state.calc(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
        }
    }
}
/// Calculates a single WAD value for one bar, updating the rolling state in place.
///
/// # Arguments
///
/// * `high` - The current bar's high price.
/// * `low` - The current bar's low price.
/// * `close` - The current bar's close price.
/// * `state` - Mutable reference to the `IndicatorState` (previous close and cumulative WAD).
///
/// # Returns
///
/// The cumulative WAD value after this bar.
#[inline(always)]
pub fn calc(high: f64, low: f64, close: f64, state: &mut IndicatorState) -> f64 {
    state.calc(high, low, close)
}
