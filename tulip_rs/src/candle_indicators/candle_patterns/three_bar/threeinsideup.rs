use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeInsideUp",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        inside_prev = "BODY"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeinsideup",
        full_name: "Three Inside Up",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanpei Fukajou",
    }
}

