/// One-Candle Shooting Star
/// Construction:
///    white or black candle with a small body
///    no lower shadow or the shadow cannot be longer than the body
///    upper shadow at least two times longer than the body
///    if the gap is created at the opening or the closing, it makes the signal stronger
///    appears as a long line
use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    common::{cdl_height, cdl_wick_length, SHORT},
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
    if (first_bar.lazy_computed & (1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT)) == 0 {
        let is_2x = cdl_wick_length((open[FIRST], close[FIRST]), high[FIRST], Some(2.0)) != SHORT;
        first_bar.set_upper_wick_2x(is_2x);
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
