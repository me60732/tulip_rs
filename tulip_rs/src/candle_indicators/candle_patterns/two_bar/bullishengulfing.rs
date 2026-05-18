use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_bar_engulf_bar,
    candle_patterns::CandlePattern
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


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
    bar(
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    let (open, _, _, close) = inputs;
    if !cdl_bar_engulf_bar((open[SECOND], close[SECOND]), (open[FIRST], close[FIRST])) {
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
