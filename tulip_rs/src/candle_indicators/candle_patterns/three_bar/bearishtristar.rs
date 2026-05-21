//! Bearish Abandoned Baby (Sute go) - Three Bar Bearish Reversal Pattern

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, PREV, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishtristar",
        full_name: "Bearish Tri-Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Santen boshi",
    }
}

#[pattern_template(
    name = "BearishTriStar",
    forecast = "BearishReversal",
    prev_bar (trend = "UP"),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN"
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (bars[THIRD].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[THIRD].apply_gap(
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
            (open[THIRD], high[THIRD], low[THIRD], close[THIRD]),
        );
    }
    if (bars[SECOND].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[SECOND].apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
    if (bars[FIRST].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[FIRST].apply_gap(
            (open[PREV], high[PREV], low[PREV], close[PREV]),
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
        );
    }
}
