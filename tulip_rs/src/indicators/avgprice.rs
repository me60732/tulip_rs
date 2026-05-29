use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::avgprice_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::avgprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the AvgPrice indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AvgPrice indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "avgprice",
        full_name: "Average Price",
        indicator_type: IndicatorType::Price,
        display_type: DisplayType::Overlay,
        inputs: &["open", "high", "low", "close"],
        options: &[],
        outputs: &["avgprice"],
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
pub fn min_data_accuracy(_options: &[f64], _decimals: usize) -> usize {
    min_data(_options)
}
/// Returns the minimum amount of data required for the AvgPrice indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AvgPrice calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Calculates the output length for the AvgPrice indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the AvgPrice calculation.
///
/// # Returns
///
/// The number of output values produced by the AvgPrice calculation.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Average Price indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — open prices
/// * `inputs[1]` — high prices
/// * `inputs[2]` — low prices
/// * `inputs[3]` — close prices
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `_options` - Unused; this indicator takes no options.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `avgprice` ((open + high + low + close) / 4),
/// and `state` can be passed to `IndicatorState::batch_indicator` for streaming.
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
fn process(inputs: &[&[f64]]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;

    let open = inputs[0];
    let high = inputs[1];
    let low = inputs[2];
    let close = inputs[3];

    let mut avgprice_line = crate::uninit_vec!(f64, open.len());

    for (i, (((&o, &h), &l), &c)) in open
        .iter()
        .zip(high.iter())
        .zip(low.iter())
        .zip(close.iter())
        .enumerate()
    {
        unsafe { *avgprice_line.get_unchecked_mut(i) = calc(o, h, l, c) };
    }

    Ok(vec![avgprice_line])
}

#[inline(always)]
pub fn calc(open: f64, high: f64, low: f64, close: f64) -> f64 {
    (open + high + low + close) * 0.25
}
