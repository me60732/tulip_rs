///Takuri Line
/// Construction:
///     white or black candle with a small body
///     no upper shadow or the shadow cannot be longer than the body
///     lower shadow at least three times longer than the body
///     if the gap is created at the opening or at the closing, it makes the signal stronger
///     appears as a long line
use crate::candle_indicators::{
    common::{cdl_height, cdl_wick_length, SHORT},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "takuriline",
        full_name: "Takuri Line",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "takuri",
    }
}

#[pattern_template(
    name = "TakuriLine",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN")
    bar(
        body_height = "SHORT",
        line_height = "LONG",
        upper_wick_lt_body = "TRUE",
        lower_wick_2x = "TRUE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // For 1-bar pattern with prev_bar:
    // bars[0] = prev_bar
    // bars[1] = current bar (the doji - already validated by registry)
    let (open, _, low, close) = inputs;

    
    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(3.0)) == SHORT { return false }

    if !(state.ema > close[FIRST] && state.ema > open[FIRST]) {
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
    let (open, _, low, close) = inputs;

    let first_bar = &mut bars[FIRST];

    
    if (first_bar.lazy_computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        first_bar.set_body_height(body_height);
    }

    if (first_bar.lazy_computed & (1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT)) == 0 {
        let is_2x = cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(2.0)) != SHORT;
        first_bar.set_lower_wick_2x(is_2x);
    }
}
