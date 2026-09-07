use crate::candle_indicators::candle_patterns::*;
use crate::indicators::candlestick::ForecastType;
use crate::types::IndicatorError;

use crate::indicators::candlestick::{CandleStick, IndicatorState, INPUTS, OPTIONS};

/// Calculates the Candlestick Pattern indicator for one asset with `N` different option sets.
///
/// This implementation calls the scalar [`indicator`] function `N` times — one per option set —
/// rather than using SIMD lanes.
///
/// # Arguments
/// * `inputs` - Shared inputs: `inputs[0]` = `open`, `inputs[1]` = `high`,
///   `inputs[2]` = `low`, `inputs[3]` = `close`.
/// * `options` - An array of `N` option sets; `options[i][0]` = candle_period,
///   `options[i][1]` = trend_period, `options[i][2]` = trend_signal_period for option set `i`.
/// * `forecast_type` - Pass `Some(ForecastType::…)` to filter detected patterns by
///   forecast direction for all option sets, or `None` to return all patterns.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][j]` is `Some(patterns)` when one or more
/// patterns are detected on bar `j` with option set `i`, or `None` otherwise,
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    forecast_type: Option<ForecastType>,
) -> Result<(Vec<Vec<Option<Vec<CandlePattern>>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no simd
    for option in options.iter() {
        let (outputs, state) = CandleStick::indicator(inputs, option, forecast_type)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
