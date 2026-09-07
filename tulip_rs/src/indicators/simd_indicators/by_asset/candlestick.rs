use crate::candle_indicators::candle_patterns::*;
use crate::indicators::candlestick::ForecastType;
use crate::types::IndicatorError;

use crate::indicators::candlestick::{CandleStick, IndicatorState, INPUTS, OPTIONS};

/// Calculates the Candlestick Pattern indicator for `N` assets by calling the scalar
/// [`indicator`] function for each asset independently.
///
/// No SIMD parallelism is used; each asset is processed sequentially.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[open, high, low, close]` for asset `i`.
/// * `options` - Shared parameter array: `options[0]` = candle_period,
///   `options[1]` = trend_period, `options[2]` = trend_signal_period.
/// * `forecast_type` - Pass `Some(ForecastType::…)` to filter detected patterns by
///   forecast direction for all assets, or `None` to return all patterns.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][j]` is `Some(patterns)` when one or more
/// patterns are detected on bar `j` of asset `i`, or `None` otherwise,
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    forecast_type: Option<ForecastType>,
) -> Result<(Vec<Vec<Option<Vec<CandlePattern>>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no roadtrain
    for input in inputs.iter() {
        let (outputs, state) = CandleStick::indicator(input, options, forecast_type)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
