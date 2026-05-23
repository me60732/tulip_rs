use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishseparatinglines",
        full_name: "Bullish Separating Lines",
        forecast: ForecastType::BullishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Iki Chigai Sen",
    }
}
#[pattern_template(
    name = "BullishSeparatingLines",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "TRUE",
        open_above_prev_mid = "TRUE",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, _) = inputs;

    if !(open[FIRST] == open[SECOND]) {
        return false;
    }
    true
}
