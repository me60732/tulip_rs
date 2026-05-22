use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "Thrusting",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        open_in_prev_body = "FALSE",
        open_above_prev_mid = "FALSE",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    )
)]
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "thrusting",
        full_name: "Thrusting",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Sashikomi",
    }
}

