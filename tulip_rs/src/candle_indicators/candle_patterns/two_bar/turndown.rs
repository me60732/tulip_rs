use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_gap,
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "turndown",
        full_name: "Turn Down",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Arawareru",
    }
}
#[pattern_template(
    name = "TurnDown",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        candle_type = "!Doji(FourPriceDoji)",
        fill = "HALLOW"
    ),
    bar(
        fill = "FILL"
        body_gap = "GAP_DOWN",
        colour = "RED",
        candle_type = "!Doji(FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, low, _) = inputs;

    
    if !(low[FIRST] < state.ema) {
        return false;
    }
    if !(open[FIRST] > state.ema) {
        return false;
    }
    if !(open[SECOND] < state.ema) {
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

    if (second_bar.lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND]));
        second_bar.set_body_gap(gap);
    }
}
