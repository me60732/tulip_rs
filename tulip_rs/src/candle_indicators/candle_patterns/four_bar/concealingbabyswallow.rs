use super::{FIRST, SECOND, THIRD};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_real_within_body,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "concealingbabyswallow",
        full_name: "Concealing Baby Swallow",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 4,
        japanese_name: "kotsubame tsutsumi",
    }
}

#[pattern_template(
    name = "ConcealingBabySwallow",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(colour = "RED", fill = "FILL", candle_type = "Marubozu(BlackMarubozu)"),
    bar(colour = "RED", fill = "FILL", candle_type = "Marubozu(BlackMarubozu)"),
    bar(
        colour = "RED",
        fill = "FILL",
        body_gap = "GAP_DOWN",
        candle_type = "SpinningTop(HighWave)"
    ),
    bar(
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        engulf_prev = "LINE"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    // FOURTH's body engulfing THIRD's full line is enforced by engulf_prev = "LINE".
    // Remaining relational checks:
    if !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
        || !cdl_real_within_body((open[SECOND], close[SECOND]), high[THIRD])
    {
        return false;
    }

    if low[THIRD] != close[THIRD] {
        return false;
    }

    true
}
