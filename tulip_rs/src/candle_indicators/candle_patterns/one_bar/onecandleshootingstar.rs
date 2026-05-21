/// One-Candle Shooting Star
/// Construction:
///    white or black candle with a small body
///    no lower shadow or the shadow cannot be longer than the body
///    upper shadow at least two times longer than the body
///    if the gap is created at the opening or the closing, it makes the signal stronger
///    appears as a long line
use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "onecandleshootingstar",
        full_name: "One-Candle Shooting Star",
        forcast: ForcastType::BearishReversal,
        bars: 1,
        japanese_name: "Nagare Boshi",
        extended_pattern: Some(CandlePattern::TwoCandleShootingStar),
    }
}

#[pattern_template(
    name = "OneCandleShootingStar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP")
    bar(
        body_height = "SHORT",
        body_gap = "GAP_UP",
        lower_wick_lt_body = "TRUE",
        upper_wick_2x = "TRUE",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    true
}
