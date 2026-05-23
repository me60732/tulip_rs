use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "Hammer",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN")
    bar(
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop)",
        line_height = "LONG",
        upper_wick_lt_body = "TRUE",
        lower_wick_lt_body = "FALSE",
        lower_wick_2x = "TRUE"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "hammer",
        full_name: "Hammer",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "kanazuchi",
    }
}

