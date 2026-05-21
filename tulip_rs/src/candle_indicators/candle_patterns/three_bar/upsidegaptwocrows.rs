use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "upsidegaptwocrows",
        full_name: "Upside Gap Two Crows",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Shita banare niwa garasu",
    }
}

#[pattern_template(
    name = "UpsideGapTwoCrows",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        body_gap = "GAP_UP",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        engulf_prev = "BODY"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    // Body engulf of SECOND by THIRD is enforced by engulf_prev = "BODY".
    // THIRD's close must still be above FIRST's close.
    if !(close[THIRD] > close[FIRST]) {
        return false;
    }

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;

    // SECOND bar: gap bits for body_gap = "GAP_UP"
    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (bars[SECOND].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[SECOND].apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }

    // THIRD bar: engulf bits for engulf_prev = "BODY"
    // Gate on I_ENGULF_PREV_BODY_BIT (bit 11) — apply_engulfing sets all of bits 1–13 atomically.
    if bars[THIRD].lazy_computed & (1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT) == 0 {
        bars[THIRD].apply_engulfing(
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
            (open[THIRD], high[THIRD], low[THIRD], close[THIRD]),
        );
    }
}
