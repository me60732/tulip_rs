//! Bearish Abandoned Baby (Sute go) - Three Bar Bearish Reversal Pattern

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_gap,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, PREV, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishtristar",
        full_name: "Bullish Tri-Star",
        forcast: ForcastType::BullishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Santen boshi",
    }
}

#[pattern_template(
    name = "BullishTriStar",
    forecast = "BullishReversal",
    prev_bar (trend = "DOWN"),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP"
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
    let (open, _, _, close) = inputs;
    if (bars[THIRD].lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[SECOND], close[SECOND]), (open[THIRD], close[THIRD]));
        bars[THIRD].set_body_gap(gap);
    }
    if (bars[SECOND].lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND]));
        bars[SECOND].set_body_gap(gap);
    }
    if (bars[FIRST].lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[PREV], close[PREV]), (open[FIRST], close[FIRST]));
        bars[FIRST].set_body_gap(gap);
    }
}
