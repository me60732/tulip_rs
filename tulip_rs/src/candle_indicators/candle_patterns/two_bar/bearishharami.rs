use crate::candle_indicators::{
    candle_patterns::CandlePattern,
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
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji)",
        inside_prev = "BODY"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    // Gate on I_ENGULF_PREV_BODY_BIT (bit 11): apply_engulfing sets all of bits 1–13
    // atomically, so if bit 11 is already in lazy_computed another call already ran.
    // This populates OPEN/CLOSE_IN_PREV_BODY (bits 2+4) checked by inside_prev = "BODY",
    // and HIGH/LOW_IN_PREV_BODY (bits 6+9) used for the haramicross rejection in calc().
    if bars[SECOND].lazy_computed & (1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT) == 0 {
        bars[SECOND].apply_engulfing(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
