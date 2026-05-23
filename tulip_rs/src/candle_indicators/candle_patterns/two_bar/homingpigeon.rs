use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "homingpigeon",
        full_name: "Homing Pigeon",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "shita banare kobato gaeshi",
    }
}

#[pattern_template(
    name = "HomingPigeon",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        candle_type = "Basic(BlackCandle | ShortBlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        inside_prev = "BODY"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Body containment is enforced by the inside_prev = "BODY" pattern mask bit.
    true
}
