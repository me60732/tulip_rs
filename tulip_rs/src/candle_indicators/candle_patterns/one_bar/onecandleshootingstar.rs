/// One-Candle Shooting Star
/// Construction:
///    white or black candle with a small body
///    no lower shadow or the shadow cannot be longer than the body
///    upper shadow at least two times longer than the body
///    if the gap is created at the opening or the closing, it makes the signal stronger
///    appears as a long line


use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::{cdl_wick_length, LONG, SHORT, cdl_height, cdl_gap},
    candle_patterns::CandlePattern
};
use tulip_rs_macros::pattern_template;

use super::{PREV, FIRST};


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
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    
    // For 1-bar pattern with prev_bar:
    // bars[0] = prev_bar
    // bars[1] = current bar (the doji - already validated by registry)

    let (open, high, low, close) = inputs;
    
    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], None) == LONG { return false }
    if cdl_wick_length((open[FIRST], close[FIRST]), high[FIRST], Some(2.5)) == SHORT { return false }
    
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;
    let first_bar = &mut bars[FIRST];
    // Ensure body_height is computed (needed by pattern template filter)
    if (first_bar.computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        first_bar.set_body_height(body_height);

    }
    // Ensure body_height is computed (needed by pattern template filter)
    if (first_bar.computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[PREV], close[PREV]), (open[FIRST], close[FIRST]));
        first_bar.set_body_gap(gap);
    }
}
