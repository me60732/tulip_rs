use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "matchinglow",
        full_name: "Matching Low",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Niten Zoko / Knuki",
    }
}

#[pattern_template(
    name = "MatchingLow",
    forecast = "BullishReversal",
    prev_bar( trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu | ClosingBlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        candle_type = "Marubozu(BlackMarubozu | ClosingBlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    if !(open[FIRST] > open[SECOND]) { 
        return false;
    }
    if !(close[FIRST] == close[SECOND]) {
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
