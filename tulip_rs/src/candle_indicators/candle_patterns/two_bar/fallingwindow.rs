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
        name: "fallingwindow",
        full_name: "Falling Window",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Shitakage",
    }
}
#[pattern_template(
    name = "FallingWindow",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        candle_type = "!Doji(FourPriceDoji)"
    ),
    bar(
        wick_gap = "GAP_DOWN",
        colour = "RED",
        candle_type = "!Doji(FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    if !(close[FIRST] < state.ema) {
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
    let (_, high, low, _) = inputs;

    let second_bar = &mut bars[SECOND];
    
    if (second_bar.lazy_computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[FIRST], low[FIRST]), (high[SECOND], low[SECOND]));
        second_bar.set_wick_gap(gap);
    }
}
