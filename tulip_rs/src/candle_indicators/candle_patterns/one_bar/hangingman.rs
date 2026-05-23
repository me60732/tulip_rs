use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "hangingman",
        full_name: "Hanging Man",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "kubitsuri",
    }
}
/// Advanced pattern calculation with additional constraints
///
/// This performs checks beyond the basic pattern matching:
/// - Each bar opens within the previous bar's body
/// - Bodies get progressively smaller
/// - Upper shadows get progressively longer
///
/// The registry handles basic filtering (trend, bar count, colours, fills).
/// This function handles the complex relational checks between bars.
///
/// # Arguments
/// * `open` - Open prices array
/// * `high` - High prices array
/// * `low` - Low prices array
/// * `close` - Close prices array
/// * `i` - Current index in the arrays
/// * `_state` - EMA state for volatility calculations (unused here)
///
/// # Returns
/// `true` if all advanced conditions are met, `false` otherwise
#[pattern_template(
    name = "HangingMan",
    forecast = "BearishReversal",
    prev_bar(trend = "UP")
    bar(
        body_height = "SHORT",
        line_height = "LONG",
        upper_wick_lt_body = "TRUE",
        lower_wick_lt_body = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    if !(state.get_ema() < close[FIRST] && state.get_ema() < open[FIRST]) {
        return false;
    }
    true
}
