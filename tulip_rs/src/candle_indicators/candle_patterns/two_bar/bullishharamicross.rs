use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishharamicross",
        full_name: "Bullish Harami Cross",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Harami yose sen",
    }
}

#[pattern_template(
    name = "BullishHaramiCross",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        candle_type = "Doji(Doji | LongLeggedDoji)",
        inside_prev = "LINE"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // FIRST's body containing SECOND's full line is enforced by inside_prev = "LINE".
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
    // This populates HIGH_IN_PREV_BODY (bit 6) and LOW_IN_PREV_BODY (bit 9)
    // which the pattern mask checks via inside_prev = "LINE".
    if bars[SECOND].lazy_computed & (1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT) == 0 {
        bars[SECOND].apply_engulfing(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
