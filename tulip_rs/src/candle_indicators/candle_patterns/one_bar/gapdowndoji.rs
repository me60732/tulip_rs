use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "GappingDownDoji",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_DOWN"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "gapdowndoji",
        full_name: "Gapping Down Doji",
        forecast: ForecastType::BearishContinuation,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Shita-hanare Doji",
    }
}

