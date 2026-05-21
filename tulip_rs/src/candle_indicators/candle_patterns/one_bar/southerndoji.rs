///Southern Doji
/// Construction:
///    a doji candle with at least one shadow
///    if the candle prior this pattern is of doji type, pattern's body has to be below it
///    if the candle prior this pattern is not a doji candle then
///        in the case of a black candle, opening on the next candle cannot be higher than the closing on the previous candle
///        in the case of a white candle, opening on the next candle cannot be higher than the opening on the previous candle
///    the high price above the low price of the previous candle
///    the high price at the level or below of the trendline
///    length of the shadows is not important
use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "sourtherndoji",
        full_name: "Southern Doji",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Kita no Doji",
    }
}

#[pattern_template(
    name = "SouthernDoji",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN")
    bar(
        colour = "RED",
        candle_type = "!Doji(FourPriceDoji)",
        body_gap = "GAP_DOWN"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, high, _, _) = inputs;

    if !(high[FIRST] <= state.ema) {
        return false;
    }

    true
}

