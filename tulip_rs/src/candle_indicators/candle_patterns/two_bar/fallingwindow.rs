use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
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
    bar(candle_type = "!Doji(FourPriceDoji)"),
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

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let bar = &mut bars[SECOND];

    if (bar.lazy_computed & (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)) == 0 {
        bar.apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
