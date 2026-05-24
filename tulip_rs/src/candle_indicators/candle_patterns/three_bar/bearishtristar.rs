use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "BearishTriStar",
    forecast = "BearishReversal",
    prev_bar (trend = "UP"),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN"
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishtristar",
        full_name: "Bearish Tri-Star",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "santen boshi",
    }
}

