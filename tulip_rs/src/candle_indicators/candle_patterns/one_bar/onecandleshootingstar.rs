/// One-Candle Shooting Star
/// Construction:
///    white or black candle with a small body
///    no lower shadow or the shadow cannot be longer than the body
///    upper shadow at least two times longer than the body
///    if the gap is created at the opening or the closing, it makes the signal stronger
///    appears as a long line
use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    common::{cdl_height, cdl_wick_length, LONG, SHORT},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, PREV};

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

    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], None) == LONG {
        return false;
    }
    if cdl_wick_length((open[FIRST], close[FIRST]), high[FIRST], Some(2.5)) == SHORT {
        return false;
    }

    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let first_bar = &mut bars[FIRST];

    if (first_bar.lazy_computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        first_bar.set_body_height(body_height);
    }

    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (first_bar.lazy_computed & body_pos_mask) != body_pos_mask {
        first_bar.apply_gap(
            (open[PREV], high[PREV], low[PREV], close[PREV]),
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
        );
    }
}
