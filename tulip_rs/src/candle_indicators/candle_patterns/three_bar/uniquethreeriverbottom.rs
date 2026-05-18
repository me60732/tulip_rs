use crate::candle_indicators::{
    common::{cdl_bar_engulf_bar, cdl_wick_length, LONG, cdl_gap},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use crate::candle_indicators::registry::CandleBits;
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
    bar( 
        colour = "RED"
        fill = "FILL", 
        line_height = "LONG",
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        line_height = "LONG",
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
    // Basic pattern matching is already done by registry:
    // - Trend is uptrend
    // - 3 bars present
    // - All bars are GREEN and HALLOW
    // - Bar 1 matches required candle types
    //
    // This function ONLY checks relational constraints between bars

    
    let (open, _, low, close) = inputs;

    // === Additional Constraints Beyond Basic Pattern Match ===

    if !cdl_bar_engulf_bar((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND])) {
        return false;
    }
    if cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], Some(2.0)) != LONG {
        return false;
    }
    if !(low[SECOND] < low[FIRST]) || !(low[THIRD] > low[SECOND]) {
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
    let (open, _, _, close) = inputs;
    let third_bar = &mut bars[THIRD];

    if (third_bar.computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[SECOND], close[SECOND]), (open[THIRD], close[THIRD]));
        third_bar.set_body_gap(gap);
    }
}
