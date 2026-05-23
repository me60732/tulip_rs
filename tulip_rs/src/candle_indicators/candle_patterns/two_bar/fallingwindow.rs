use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;

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

    if !(close[FIRST] < state.get_ema()) {
        return false;
    }

    true
}
