use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "KickingUp",
    forecast = "BullishReversalOrContinuation",
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        wick_gap = "GAP_UP",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "kickingup",
        full_name: "Kicking Up",
        forcast: ForcastType::BullishReversalOrContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Keri Ashi",
    }
}



