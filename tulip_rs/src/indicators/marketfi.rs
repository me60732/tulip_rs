use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::marketfi_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::marketfi_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}

//#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS]) -> Result<Vec<Vec<f64>>, IndicatorError> {
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

pub struct Marketfi;
impl Indicator<INPUTS, OPTIONS> for Marketfi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "marketfi",
        indicator_type: IndicatorType::Volume,
        full_name: "Market Facilitation Index",
        inputs: &["high", "low", "volume"],
        options: &[],
        outputs: &["marketfi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "marketfi",
            label: "MARKETFI",
            display_type: DisplayType::Indicator,
            outputs: &["marketfi"],
        }],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        1
    }
    
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
