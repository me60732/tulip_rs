use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "turndown",
        full_name: "Turn Down",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Kotowaru",
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
    low[FIRST] < state.get_ema() && open[FIRST] > state.get_ema() && open[SECOND] < state.get_ema()
}
