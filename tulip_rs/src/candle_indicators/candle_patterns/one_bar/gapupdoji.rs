use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "GappingUpDoji",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_UP"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "gapupdoji",
        full_name: "Gapping Up Doji",
        extended_pattern: None,
        forecast: ForecastType::BullishContinuation,
        bars: 1,
        japanese_name: "Ue-hanare Doji",
    }
}

