use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "matchinghigh",
        full_name: "Matching High",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Niten Tenjo",
    }
}

#[pattern_template(
    name = "MatchingHigh",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu | ClosingWhiteMarubozu)"
    ),
    bar(
        inside_prev = "BODY",
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "Marubozu(WhiteMarubozu | ClosingWhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    close[FIRST] == close[SECOND]
}
