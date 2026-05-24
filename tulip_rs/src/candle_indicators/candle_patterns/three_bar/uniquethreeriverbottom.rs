use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "UniqueThreeRiverBottom",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        line_height = "LONG",
        inside_prev = "BODY",
        lower_wick_2x = "TRUE",
        low_in_prev_line = "FALSE",
    ),
    bar(
        colour = "RED",
        fill = "HALLOW",
        line_height = "SHORT",
        body_gap = "GAP_DOWN",
        low_in_prev_line = "TRUE"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "uniquethreeriverbottom",
        full_name: "Unique Three River Bottom",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: " sankawa soko zuka",
    }
}



