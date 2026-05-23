use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishmeetinglines",
        full_name: "Bullish Meeting Lines",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Deaisen",
    }
}
#[pattern_template(
    name = "BullishMeetingLines",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    if !(close[FIRST] == close[SECOND]) {
        return false;
    }
    true
}
