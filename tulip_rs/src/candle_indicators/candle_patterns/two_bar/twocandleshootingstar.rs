///Southern Doji
/// Construction:
///    First candle
///     a candle in an uptrend
///     white body
///    Second candle
///     a doji candle
///     a body above the first candle's body
use crate::candle_indicators::{
    common::cdl_wick_length,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::SECOND;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "twocandleshootingstar",
        full_name: "Two Candle Shooting Star",
        forecast: ForecastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Nagare Boshi",
    }
}

#[pattern_template(
    name = "TwoCandleShootingStar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        body_height = "SHORT",
        body_gap = "GAP_UP",
        upper_wick_lt_body = "FALSE",
        upper_wick_2x = "TRUE", 
        lower_wick_lt_body = "TRUE",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, _, close) = inputs;
    cdl_wick_length((open[SECOND], close[SECOND]), high[SECOND], Some(2.5))
}
