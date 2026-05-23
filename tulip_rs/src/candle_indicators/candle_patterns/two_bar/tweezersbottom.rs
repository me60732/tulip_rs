use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "tweezersbottom",
        full_name: "Tweezers Bottom",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Kenukizoko",
    }
}

#[pattern_template(
    name = "TweezersBottom",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(candle_type = "!Doji(FourPriceDoji)"),
    bar(
        candle_type = "!Doji(FourPriceDoji)",
        inside_prev = "LINE"
    ),
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, low, _) = inputs;

    low[FIRST] == low[SECOND]
}
