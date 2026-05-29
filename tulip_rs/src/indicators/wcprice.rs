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
pub use crate::indicators::simd_indicators::wcprice_simd::indicator_by_assets;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::wcprice_simd::indicator_by_assets as indicator;
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
pub fn info() -> Info<'static> {
    Info {
        name: "wcprice",
        full_name: "Weighted Close Price",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Price,
        // Use only the necessary inputs: high, low, close.
        inputs: &["high", "low", "close"],
        // No options.
        options: &[],
        outputs: &["wcprice"],
        // No state required for this indicator.
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
/// * `options` - A slice containing the indicator options (unused; wcprice takes no options).
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Weighted Close Price indicator.
///
/// # Arguments
///
/// * `_options` - Unused; wcprice takes no options.
///
/// # Returns
///
/// The minimum amount of data required (1; each bar is computed independently).
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - Unused; wcprice takes no options.
///
/// # Returns
///
/// The output length (equal to `data_len`).
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Weighted Close Price indicator over the full input dataset.
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
/// * `_options` - Unused; wcprice takes no options.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `wcprice`. The returned
/// `IndicatorState` is stateless and may be passed to `batch_indicator`
/// for subsequent calls. Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    let outputs = process(inputs)?;
    Ok((outputs, IndicatorState))
}
//#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS_WIDTH]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut wcprice_line = crate::uninit_vec!(f64, inputs[0].len());

    for i in 0..high.len() {
        unsafe {
            *wcprice_line.get_unchecked_mut(i) = calc(
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            )
        };
    }

    Ok(vec![wcprice_line])
}

/// Calculates the Weighted Close Price for a single bar.
///
/// Computes `(high + low + 2 * close) / 4`.
///
/// # Arguments
///
/// * `high` - Reference to the current bar's high price.
/// * `low` - Reference to the current bar's low price.
/// * `close` - Reference to the current bar's close price.
///
/// # Returns
///
/// The weighted close price for this bar.
#[inline(always)]
pub fn calc(high: &f64, low: &f64, close: &f64) -> f64 {
    close.mul_add(2.0, high + low) * 0.25
}
