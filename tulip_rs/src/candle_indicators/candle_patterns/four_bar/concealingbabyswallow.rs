use super::{FIRST, FOURTH, SECOND, THIRD};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_bar_engulf_bar, cdl_real_within_body},
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
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    // === Additional Constraints Beyond Basic Pattern Match ===
    if !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
        || !cdl_real_within_body((open[SECOND], close[SECOND]), high[THIRD])
    {
        return false;
    }

    if !cdl_bar_engulf_bar((open[FOURTH], close[FOURTH]), (low[THIRD], high[THIRD])) {
        return false;
    }

    if low[THIRD] != close[THIRD] {
        return false;
    }
    // All conditions met
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;

    let third_bar = &mut bars[3];
    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (third_bar.lazy_computed & body_pos_mask) != body_pos_mask {
        third_bar.apply_gap(
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
            (open[THIRD], high[THIRD], low[THIRD], close[THIRD]),
        );
    }
}
