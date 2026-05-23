use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "downsidetasukigap",
        full_name: "Downside Tasuki Gap",
        forecast: ForecastType::BearishContinuation,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Shita banare tasuki",
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
    name = "DownsideTasukiGap",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        wick_gap = "GAP_DOWN",
        candle_type = "Basic(BlackCandle | LongBlackCandle | ShortBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    let (_, _, _, close) = inputs;

    close[THIRD] < close[FIRST]
}
