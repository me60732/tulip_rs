use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "BearishHarami",
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
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji)",
        inside_prev = "BODY"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishharami",
        full_name: "Bearish Harami",
        forecast: ForecastType::BearishReversal,
        extended_pattern: Some(CandlePattern::ThreeInsideDown),
        bars: 2,
        japanese_name: "Harami",
    }
}



