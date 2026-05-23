use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishharamicross",
        full_name: "Bullish Harami Cross",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Harami yose sen",
    }
}

#[pattern_template(
    name = "BullishHaramiCross",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        candle_type = "Doji(Doji | LongLeggedDoji)",
        inside_prev = "LINE"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // FIRST's body containing SECOND's full line is enforced by inside_prev = "LINE".
    true
}
