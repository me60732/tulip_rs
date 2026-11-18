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

#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let mut typprice_line = crate::uninit_vec!(f64, high.len()); // Vec::with_capacity(capacity);

    cycle_typprice((high, low, close), &mut typprice_line);

    Ok(vec![typprice_line])
}
/// Performs the main calculation loop for the TYPPRICE indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of slices `(high, low, close)` containing the price data.
/// * `typprice_line` - A mutable slice for storing the TYPPRICE output values.
#[inline(always)]
fn cycle_typprice(inputs: (&[f64], &[f64], &[f64]), typprice_line: &mut [f64]) {
    let (high, low, close) = inputs;
    for i in 0..high.len() {
        unsafe {
            *typprice_line.get_unchecked_mut(i) = calc(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
        }
    }
}

/// Calculates the Typical Price (TYPPRICE) value.
///
/// # Arguments
///
/// * `high` - The high price.
/// * `low` - The low price.
/// * `close` - The close price.
///
/// # Returns
///
/// The TYPPRICE value.
const DIV: f64 = 1.0 / 3.0;
#[inline(always)]
pub fn calc(high: f64, low: f64, close: f64) -> f64 {
    (high + low + close) * DIV
}

pub struct Typprice;
impl Indicator<INPUTS, OPTIONS> for Typprice {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "typprice",
        full_name: "Typical Price",
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["typprice"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "typprice",
            label: "TYPPRICE",
            display_type: DisplayType::Overlay,
            outputs: &["typprice"],
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
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        let outputs = process(inputs)?;
        Ok((outputs, IndicatorState))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::typprice_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
