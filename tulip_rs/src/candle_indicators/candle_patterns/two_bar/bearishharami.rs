use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    common::cdl_bar_engulf_bar,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishharami",
        full_name: "Bearish Harami",
        forcast: ForcastType::BearishReversal,
        extended_pattern: Some(CandlePattern::ThreeInsideDown),
        bars: 2,
        japanese_name: "Harami",
    }
}

#[pattern_template(
    name = "BearishHarami",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(colour = "RED", fill = "FILL", candle_type = "!Doji(FourPriceDoji)",)
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    if high[FIRST] == high[SECOND] {
        return false;
    }
    // Fast bit check: if current bar is any Doji type (FourPriceDoji already filtered by template)
    // Check for any doji variant using the public constants
    let second_bar = bars[SECOND].mandatory;
    let is_doji = (second_bar & CandleBits::DOJI) != 0
        || (second_bar & CandleBits::LONG_LEGGED_DOJI) != 0
        || (second_bar & CandleBits::DRAGONFLY_DOJI) != 0
        || (second_bar & CandleBits::GRAVESTONE_DOJI) != 0;

    // === Additional Constraints Beyond Basic Pattern Match ===
    if is_doji {
        // For doji: check if first bar's body engulfs current bar's full range
        if cdl_bar_engulf_bar((open[FIRST], close[FIRST]), (high[SECOND], low[SECOND])) {
            return false;
        }
    }

    // First bar's body must engulf second bar's body
    if !cdl_bar_engulf_bar((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND])) {
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
