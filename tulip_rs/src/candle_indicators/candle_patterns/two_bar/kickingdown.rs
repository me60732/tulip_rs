use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_gap
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "kickingdown",
        full_name: "Kicking Down",
        forcast: ForcastType::BearishReversalOrContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Keri Ashi",
    }
}

#[pattern_template(
    name = "KickingDown",
    forecast = "BearishReversalOrContinuation",
    bar(
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu)"
    ),
    bar(
        body_gap = "GAP_DOWN",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, _) = inputs;
    
    if !(open[FIRST] > open[SECOND]) {
        
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
    let (open, _, _, close) = inputs;
    let second_bar = &mut bars[SECOND];

    if (second_bar.computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND]));
        second_bar.set_body_gap(gap);
    }
}
