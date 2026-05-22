use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeInsideDown",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        inside_prev = "BODY"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeinsidedown",
        full_name: "Three Inside Down",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanpei Fukakudari",
    }
}



