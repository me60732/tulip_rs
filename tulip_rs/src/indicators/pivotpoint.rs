use crate::common::{validate_inputs, validate_options};
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
pub const OPTIONS: usize = 1;

pub struct PivotPoint;
impl Indicator<INPUTS, OPTIONS> for PivotPoint {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "pivotpoint",
        full_name: "Pivot Point",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["s3", "s2", "s1", "pp", "r1", "r2", "r3"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "pivotpoint",
            label: "PIVOTPOINT",
            display_type: DisplayType::Overlay,
            outputs: &["s3", "s2", "s1", "pp", "r1", "r2", "r3"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn output_length(_data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        validate_inputs(inputs, Self::min_data(options))?;

        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];
        let outputs = process(high, low, close, period);

        Ok((
            outputs,
            IndicatorState {
                period,
                high: high[high.len() - period + 1..].to_vec(),
                low: low[low.len() - period + 1..].to_vec(),
                close: close[close.len() - period + 1..].to_vec(),
            },
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::pivotpoint_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for PivotPoint {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::pivotpoint_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);
        self.close.extend_from_slice(inputs[2]);
        let outputs = process(&self.high, &self.low, &self.close, self.period);

        self.high.drain(..self.high.len() - self.period + 1);
        self.low.drain(..self.low.len() - self.period + 1);
        self.close.drain(..self.close.len() - self.period + 1);

        Ok(outputs)
    }
}

fn process(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<Vec<f64>> {
    let start_index = high.len() - period;
    let high = &high[start_index..];
    let low = &low[start_index..];
    let close = &close[start_index..];
    let (s3, s2, s1, pp, r1, r2, r3) = calc(high, low, close);
    vec![vec![s3, s2, s1, pp, r1, r2, r3]]
}

/// Calculates the support and resistance levels for the Pivot Point indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices over the look-back period.
/// * `low` - A slice of low prices over the look-back period.
/// * `close` - A slice of close prices; the last element is used as the closing price.
///
/// # Returns
///
/// A tuple `(s3, s2, s1, pivot_point, r1, r2, r3)` of the three support levels,
/// the pivot point, and the three resistance levels.
#[inline(always)]
pub fn calc(high: &[f64], low: &[f64], close: &[f64]) -> (f64, f64, f64, f64, f64, f64, f64) {
    let close_value = close[close.len() - 1];
    let (low_value, high_value) = low
        .iter()
        .copied()
        .zip(high.iter().copied())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), (l, h)| {
            (min.min(l), max.max(h))
        });

    let pivot_point = (high_value + low_value + close_value) / 3.0;
    let s1 = (pivot_point * 2.0) - high_value;
    let s2 = pivot_point - (high_value - low_value);
    let s3 = s1 - (high_value - low_value);
    let r1 = (pivot_point * 2.0) - low_value;
    let r2 = pivot_point + (high_value - low_value);
    let r3 = r1 + (high_value - low_value);
    (s3, s2, s1, pivot_point, r1, r2, r3)
}
