use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "turnup",
        full_name: "Turn UP",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Arawareru",
    }
}
#[pattern_template(
    name = "TurnUp",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(candle_type = "!Doji(FourPriceDoji)", fill = "FILL"),
    bar(
        fill = "HALLOW",
        body_gap = "GAP_UP",
        colour = "GREEN",
        candle_type = "!Doji(FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, _, _) = inputs;
    high[FIRST] > state.get_ema() && open[FIRST] < state.get_ema() && open[SECOND] > state.get_ema()
}
