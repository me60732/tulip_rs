use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "descendinghawk",
        full_name: "Descending Hawk",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "kakouchu no taka",
    }
}

#[pattern_template(
    name = "DescendingHawk",
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
        fill = "HALLOW",
        candle_type = "Basic(WhiteCandle | ShortWhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        inside_prev = "BODY"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Body containment is enforced by the inside_prev = "BODY" pattern mask bit.
    true
}
