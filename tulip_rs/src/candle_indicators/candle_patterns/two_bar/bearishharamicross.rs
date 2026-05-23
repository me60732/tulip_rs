use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishharami",
        full_name: "Bearish Harami Cross",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Harami yose sen",
    }
}

#[pattern_template(
    name = "BearishHaramiCross",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
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
