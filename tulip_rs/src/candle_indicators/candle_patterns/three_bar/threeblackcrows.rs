use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeBlackCrows",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
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
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "TRUE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "TRUE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeblackcrows",
        full_name: "Three Black Crows",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanba Garasu",
    }
}



