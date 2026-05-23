use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "BearishDojiStar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "HALLOW"
        colour = "GREEN",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        body_gap = "GAP_UP"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishdojistar",
        full_name: "Bearish Doji Star",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Doji Bike",
    }
}



