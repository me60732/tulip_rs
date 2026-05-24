use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::medprice_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::medprice_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}
/// Returns information about the Median Price (MEDPRICE) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MEDPRICE indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "medprice",
        full_name: "Median Price",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low"],
        options: &[],
        outputs: &["medprice"],
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
/// Returns the minimum amount of data required for the MEDPRICE indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the MEDPRICE calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1 // Only one data point is needed to calculate the median price
}

/// Calculates the output length for the MEDPRICE indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the MEDPRICE calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Median Price (MEDPRICE) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Unused; pass `&[]` (this indicator has no options).
/// * `optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `medprice`
///
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

    let mut medprice_line = crate::uninit_vec!(f64, high.len());

    for (i, (&high_value, &low_value)) in high.iter().zip(low.iter()).enumerate() {
        unsafe { *medprice_line.get_unchecked_mut(i) = calc(high_value, low_value) };
    }

    Ok(vec![medprice_line])
}

/// Calculates the median price.
#[inline(always)]
pub fn calc(high: f64, low: f64) -> f64 {
    0.5 * (high + low)
}
