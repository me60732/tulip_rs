use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "kickingup",
        full_name: "Kicking Up",
        forcast: ForcastType::BullishReversalOrContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Keri Ashi",
    }
}

#[pattern_template(
    name = "KickingUp",
    forecast = "BullishReversalOrContinuation",
    bar(
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, _) = inputs;

    if !(open[FIRST] < open[SECOND]) {
        return false;
    }

    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let second_bar = &mut bars[SECOND];

    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (second_bar.lazy_computed & body_pos_mask) != body_pos_mask {
        second_bar.apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
