use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "matchinglow",
        full_name: "Matching Low",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Niten Zoko / Knuki",
    }
}

#[pattern_template(
    name = "MatchingLow",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu | ClosingBlackMarubozu)"
    ),
    bar(
        inside_prev = "BODY",
        colour = "GREEN",
        fill = "FILL",
        candle_type = "Marubozu(BlackMarubozu | ClosingBlackMarubozu)"
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
