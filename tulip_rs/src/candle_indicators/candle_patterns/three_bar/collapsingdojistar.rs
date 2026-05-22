use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;


#[pattern_template(
    name = "Collapsingdojistar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "HALLOW",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
    bar(
        colour = "RED",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_DOWN"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        wick_gap = "GAP_DOWN"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "collapsingdojistar",
        full_name: "Collapsing Doji Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Hōkai suru dōjī sutā",
    }
}


