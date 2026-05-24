use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeOutsideDown",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(FourPriceDoji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        engulf_prev = "BODY"
    ),
    bar(colour = "RED", fill = "FILL")
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeoutsidedown",
        full_name: "Three Outside Down",
        forecast: ForecastType::BearishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Sanpei Gaishi",
    }
}

