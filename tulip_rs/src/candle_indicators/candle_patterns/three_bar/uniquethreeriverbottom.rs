use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_wick_length, LONG},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "uniquethreeriverbottom",
        full_name: "Unique Three River Bottom",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sankawa soko zuka",
    }
}

#[pattern_template(
    name = "UniqueThreeRiverBottom",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(colour = "RED", fill = "FILL", line_height = "LONG",),
    bar(
        colour = "GREEN",
        fill = "FILL",
        line_height = "LONG",
        inside_prev = "BODY"
    ),
    bar(
        colour = "RED",
        fill = "HALLOW",
        line_height = "SHORT",
        body_gap = "GAP_DOWN",
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, low, close) = inputs;

    // Body containment of SECOND inside FIRST is enforced by inside_prev = "BODY".
    // Remaining relational checks:
    if cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], Some(2.0)) != LONG {
        return false;
    }
    if !(low[SECOND] < low[FIRST]) || !(low[THIRD] > low[SECOND]) {
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

    // SECOND bar: engulf bits for inside_prev = "BODY"
    // Gate on I_ENGULF_PREV_BODY_BIT (bit 11) — apply_engulfing sets all of bits 1–13 atomically.
    if bars[SECOND].lazy_computed & (1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT) == 0 {
        bars[SECOND].apply_engulfing(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }

    // THIRD bar: gap bits for body_gap = "GAP_DOWN"
    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (bars[THIRD].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[THIRD].apply_gap(
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
            (open[THIRD], high[THIRD], low[THIRD], close[THIRD]),
        );
    }
}
