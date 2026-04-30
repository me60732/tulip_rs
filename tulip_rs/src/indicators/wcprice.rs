use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wcprice_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
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
        indicator_type: IndicatorType::Trend,
        // Use only the necessary inputs: high, low, close.
        inputs: &["high", "low", "close"],
        // No options.
        options: &[],
        outputs: &["wcprice"],
        // No state required for this indicator.
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum required data points (each bar is computed independently).
pub fn min_data(_options: &[f64]) -> usize {
    1
}

/// Returns the output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Full-indicator calculation for wcprice.

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

/// Per-bar calculation for wcprice.
/// Computes (high + low + 2 * close) / 4.
#[inline(always)]
pub fn calc(high: &f64, low: &f64, close: &f64) -> f64 {
    close.mul_add(2.0, high + low) * 0.25
}
