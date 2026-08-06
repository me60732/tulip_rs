use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

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
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
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

pub struct AvgPrice;
impl Indicator<INPUTS, OPTIONS> for AvgPrice {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "avgprice",
        full_name: "Average Price",
        indicator_type: IndicatorType::Price,
        inputs: &["open", "high", "low", "close"],
        options: &[],
        outputs: &["avgprice"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "avgprice",
            label: "AVGPRICE",
            display_type: DisplayType::Overlay,
            outputs: &["avgprice"],
        }],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
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
    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        let outputs = process(inputs)?;
        Ok((outputs, IndicatorState))
    }
}