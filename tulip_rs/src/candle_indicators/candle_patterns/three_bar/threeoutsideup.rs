use crate::candle_indicators::types::{CandleInfo, ForecastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "ThreeOutsideUp",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(colour = "RED", fill = "FILL", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        engulf_prev = "BODY"
    ),
    bar(colour = "GREEN", fill = "HALLOW",)
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeoutsideup",
        full_name: "Three Outside Up",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanpei Gaishi",
    }
}


