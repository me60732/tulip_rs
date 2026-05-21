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
        name: "bullishharami",
        full_name: "Bullish Harami",
        forcast: ForcastType::BullishReversal,
        extended_pattern: Some(CandlePattern::ThreeInsideUp),
        bars: 2,
        japanese_name: "Harami",
    }
}

#[pattern_template(
    name = "BullishHarami",
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
        fill = "HALLOW",
        candle_type = "!Doji(FourPriceDoji)",
        inside_prev = "BODY"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &[CandleBits],
) -> bool {
    let (_, _, low, _) = inputs;

    // Sharing the same low means the lines touch — not a true harami
    if low[FIRST] == low[SECOND] {
        return false;
    }

    // If second bar is a doji, exclude the haramicross case:
    // when FIRST's body also contains SECOND's full line (high and low),
    // that is a haramicross, not a harami.
    let second_mandatory = bars[SECOND].mandatory;
    let is_doji = (second_mandatory & CandleBits::DOJI) != 0
        || (second_mandatory & CandleBits::LONG_LEGGED_DOJI) != 0
        || (second_mandatory & CandleBits::DRAGONFLY_DOJI) != 0
        || (second_mandatory & CandleBits::GRAVESTONE_DOJI) != 0;

    if is_doji {
        // apply_engulfing (run in compute_bits) sets HIGH_IN_PREV_BODY and LOW_IN_PREV_BODY.
        // If both are true, FIRST's body fully contains SECOND's range → haramicross → reject.
        let high_in_prev_body =
            bars[SECOND].lazy_value & (1u16 << CandleBits::HIGH_IN_PREV_BODY_BIT) != 0;
        let low_in_prev_body =
            bars[SECOND].lazy_value & (1u16 << CandleBits::LOW_IN_PREV_BODY_BIT) != 0;
        if high_in_prev_body && low_in_prev_body {
            return false;
        }
    }

    true
}
