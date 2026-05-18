use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_real_within_body
};
use tulip_rs_macros::pattern_template;

use super::{PREV, FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishtasukiline",
        full_name: "Bullish Tasuki Line",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Tasuki",
    }
}
#[pattern_template(
    name = "BullishTasukiLine",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    if !(high[FIRST] < low[PREV]) {
        return false;
    }

    if !(close[SECOND] > open[FIRST]) {
        return false;
    }
    
    if !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND]) {
        return false;
    }
    
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &mut [CandleBits],
) {
    // No lazy bits needed for this pattern
}
