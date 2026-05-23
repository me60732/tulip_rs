
use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "BullishDojiStar",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL"
        colour = "RED",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        body_gap = "GAP_DOWN"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishdojistar",
        full_name: "Bullish Doji Star",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Doji Bike",
    }
}



