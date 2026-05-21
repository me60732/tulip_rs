use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
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
