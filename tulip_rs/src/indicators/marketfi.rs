use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::marketfi_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::marketfi_simd::indicator_by_assets as indicator;
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
/// Returns information about the Market Facilitation Index (MarketFI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MarketFI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "marketfi",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Volatility,
        full_name: "Market Facilitation Index",
        inputs: &["high", "low", "volume"],
        options: &[],
        outputs: &["marketfi"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(_options: &[f64], _decimals: usize) -> usize {
    min_data(_options)
}
/// Returns the minimum amount of data required for the MarketFI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the MarketFI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the MarketFI calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Market Facilitation Index (MarketFI) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the MarketFI calculation.
///
/// # Returns
///
/// A vector of vectors containing the MarketFI line.

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
    let volume = inputs[2];

    let mut marketfi_line = crate::uninit_vec!(f64, high.len());

    // Perform the main MarketFI calculation
    for i in 0..high.len() {
        unsafe {
            *marketfi_line.get_unchecked_mut(i) = calc(
                high.get_unchecked(i),
                low.get_unchecked(i),
                volume.get_unchecked(i),
            )
        };
    }

    Ok(vec![marketfi_line])
}

#[inline(always)]
pub fn calc(high: &f64, low: &f64, volume: &f64) -> f64 {
    (high - low) / volume.max(f64::EPSILON)
}
