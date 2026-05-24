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
pub use crate::indicators::simd_indicators::ad_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ad_simd::indicator_by_assets as indicator;
}
/// Returns information about the Accumulation/Distribution Line (AD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AD indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "ad",
        full_name: "Accumulation/Distribution Line",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close", "volume"],
        options: &[],
        outputs: &["ad"],
        optional_outputs: &[],
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct IndicatorState {
    ad: f64,
}
impl IndicatorState {
    pub fn new(ad: f64) -> Self {
        Self { ad }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    //#[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut ad_line = crate::uninit_vec!(f64, inputs[0].len());

        self.ad = cycle(inputs, &mut ad_line, self.ad);

        Ok(vec![ad_line])
    }
}
/// Returns the minimum amount of data required for the AD indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AD calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    1
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimal_places` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimal_places: usize) -> usize {
    min_data(options)
}
/// Calculates the output length for the AD indicator based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the AD calculation (unused; AD has no options).
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Accumulation/Distribution Line (AD) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
/// * `inputs[3]` — volume
///
/// # Arguments
///
/// * `inputs` - Array of 4 input price/volume slices (see Inputs above).
/// * `_options` - Unused; AD has no configurable options.
/// * `_optional_outputs` - Unused; AD produces no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `ad` line and
/// `state` can be passed to `IndicatorState::batch_indicator` to continue streaming.
///
/// Returns `Err(IndicatorError)` if inputs are too short.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let mut ad_line = crate::uninit_vec!(f64, inputs[0].len());

    let mut ad = 0.0;
    ad = cycle(inputs, &mut ad_line, ad);

    Ok((vec![ad_line], IndicatorState { ad }))
}

/// Performs the main calculation loop for the AD indicator.
///
/// # Arguments
///
/// * `inputs` - A reference to an array of 4 input slices: high, low, close, and volume.
/// * `ad_line` - A mutable slice for storing the resulting AD line values.
/// * `ad` - The running AD accumulator value to continue from.
///
/// # Returns
///
/// The final AD accumulator value after processing all inputs.
#[inline(always)]
fn cycle(inputs: &[&[f64]; INPUTS_WIDTH], ad_line: &mut [f64], mut ad: f64) -> f64 {
    let (high, low, close, volume) = (inputs[0], inputs[1], inputs[2], inputs[3]);
    for i in 0..high.len() {
        unsafe {
            ad = calc(
                ad,
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            );
            *ad_line.get_unchecked_mut(i) = ad;
        };
    }
    ad
}

/// Calculates the current value of the Accumulation/Distribution Line (AD) indicator.
///
/// # Arguments
///
/// * `ad` - The previous AD value.
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `volume` - The current volume.
///
/// # Returns
///
/// The updated AD value.
#[inline(always)]
pub fn calc(ad: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
    let range = high - low;
    if range <= f64::EPSILON {
        return ad;
    }

    //ad + (close - low - high + close) / range * volume
    ((close - low - high + close) / range).mul_add(volume, ad)
}
