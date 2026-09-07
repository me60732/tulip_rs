use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

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

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::marketfi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
