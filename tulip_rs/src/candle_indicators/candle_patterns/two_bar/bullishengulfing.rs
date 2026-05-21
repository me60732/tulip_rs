use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishengulfing",
        full_name: "Bullish Engulfing",
        forcast: ForcastType::BullishReversal,
        extended_pattern: Some(CandlePattern::ThreeOutsideUp),
        bars: 2,
        japanese_name: "Tsutsumi",
    }
}

#[pattern_template(
    name = "BullishEngulfing",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(fill = "FILL", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        engulf_prev = "BODY"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Body engulf is enforced by the engulf_prev = "BODY" pattern mask bit.
    true
}
