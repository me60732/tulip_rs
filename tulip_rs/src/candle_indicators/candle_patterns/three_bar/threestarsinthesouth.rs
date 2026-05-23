use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeStarsInTheSouth",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED", 
        fill = "FILL",
        line_height = "LONG",
        lower_wick_lt_body = "FALSE"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        open_in_prev_body = "TRUE",
        low_in_prev_line = "TRUE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        fill = "FILL",
        line_height = "SHORT",
        inside_prev = "LINE",
        candle_type = "Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threestarsinthesouth",
        full_name: "Three Stars In The South",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Kyoku no santen boshi",
    }
}



