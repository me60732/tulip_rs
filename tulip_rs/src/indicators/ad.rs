use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 4;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ad_simd::indicator_by_assets;
#[cfg(feature = "simd_assets")]
pub mod by_assets {
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
pub fn min_data_accuracy(options: &[f64], _decimal_places: usize) -> usize {
    min_data(options)
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the AD calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len
}

/// Calculates the Accumulation/Distribution Line (AD) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, close prices, and volume.
/// * `_options` - A slice containing the options for the AD calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data, keep in mind with most indicators this is speed vs accuracy.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A `Result` containing a vector of vectors with the AD line or an `IndicatorError`.

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
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `volume` - A slice of volume data.
/// * `ad_line` - A mutable reference to a vector for storing the AD line.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
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
/// * `prev_ad` - The previous AD value.
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
