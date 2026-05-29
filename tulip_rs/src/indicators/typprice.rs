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
pub use crate::indicators::simd_indicators::typprice_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::typprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the Typical Price (TYPPRICE) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TYPPRICE indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "typprice",
        full_name: "Typical Price",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["typprice"],
        optional_outputs: &[],
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
/// Returns the minimum amount of data required for the TYPPRICE indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the TYPPRICE calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the TYPPRICE calculation (unused).
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Typical Price (TYPPRICE) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `_options` - Unused; TYPPRICE has no options.
/// * `_optional_outputs` - Unused; TYPPRICE has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `typprice` and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    let outputs = process(inputs)?;
    Ok((outputs, IndicatorState))
}
#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS_WIDTH]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut typprice_line = crate::uninit_vec!(f64, high.len()); // Vec::with_capacity(capacity);

    cycle_typprice((high, low, close), &mut typprice_line);

    Ok(vec![typprice_line])
}
/// Performs the main calculation loop for the TYPPRICE indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of slices `(high, low, close)` containing the price data.
/// * `typprice_line` - A mutable slice for storing the TYPPRICE output values.
#[inline(always)]
fn cycle_typprice(inputs: (&[f64], &[f64], &[f64]), typprice_line: &mut [f64]) {
    let (high, low, close) = inputs;
    for i in 0..high.len() {
        unsafe {
            *typprice_line.get_unchecked_mut(i) = calc(
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            );
        }
    }
}

/// Calculates the Typical Price (TYPPRICE) value.
///
/// # Arguments
///
/// * `high` - The high price.
/// * `low` - The low price.
/// * `close` - The close price.
///
/// # Returns
///
/// The TYPPRICE value.
const DIV: f64 = 1.0 / 3.0;
#[inline(always)]
pub fn calc(high: &f64, low: &f64, close: &f64) -> f64 {
    (high + low + close) * DIV
}
