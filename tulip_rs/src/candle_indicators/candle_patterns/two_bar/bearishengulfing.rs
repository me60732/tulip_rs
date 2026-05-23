use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishengulfing",
        full_name: "Bearish Engulfing",
        forecast: ForecastType::BearishReversal,
        extended_pattern: Some(CandlePattern::ThreeOutsideDown),
        bars: 2,
        japanese_name: "Tsutsumi",
    }
}

#[pattern_template(
    name = "BearishEngulfing",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(fill = "HALLOW", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
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
