///Northern Doji
/// Construction:
///    a doji candle with at least one shadow
///    if the candle prior this pattern is of doji type, pattern's body has to be above it
///    if the candle prior this pattern is not a doji candle then
///        in the case of a black candle, opening on the next candle cannot be lower than the opening on the previous candle
///        in the case of a white candle, opening on the next candle cannot be lower than the closing on the previous candle
///    the low price below the high price of the previous candle
///    the low price at the level or above of the trendline
///    length of the shadows is not important
use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "northerndoji",
        full_name: "Northern Doji",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Kita no Doji",
    }
}

#[pattern_template(
    name = "NorthernDoji",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        candle_type = "Doji(LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)",
        body_gap = "GAP_UP"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, low, _) = inputs;
    low[FIRST] >= state.get_ema()
}
