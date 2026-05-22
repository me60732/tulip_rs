use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "BullishTriStar",
    forecast = "BullishReversal",
    prev_bar (trend = "DOWN"),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP"
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishtristar",
        full_name: "Bullish Tri-Star",
        forcast: ForcastType::BullishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Santen boshi",
    }
}



