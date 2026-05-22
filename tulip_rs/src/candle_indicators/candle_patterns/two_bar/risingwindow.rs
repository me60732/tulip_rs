use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "risingwindow",
        full_name: "Rising Window",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Shitakage",
    }
}
#[pattern_template(
    name = "RisingWindow",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(candle_type = "!Doji(FourPriceDoji)"),
    bar(
        wick_gap = "GAP_UP",
        colour = "GREEN",
        candle_type = "!Doji(FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    close[FIRST] > state.ema
}
