use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "DarkCloudCover",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "FALSE",
        open_above_prev_mid = "TRUE",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "darkcloudcover",
        full_name: "Dark Cloud Cover",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Kumo no Ura",
    }
}


