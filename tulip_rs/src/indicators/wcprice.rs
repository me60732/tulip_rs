use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState,
};
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

pub struct WcPrice;
impl Indicator<INPUTS, OPTIONS> for WcPrice {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "wcprice",
        full_name: "Weighted Close Price",
        indicator_type: IndicatorType::Price,
        // Use only the necessary inputs: high, low, close.
        inputs: &["high", "low", "close"],
        // No options.
        options: &[],
        outputs: &["wcprice"],
        // No state required for this indicator.
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "wcprice",
            label: "WCPRICE",
            display_type: DisplayType::Overlay,
            outputs: &["wcprice"],
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
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wcprice_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
