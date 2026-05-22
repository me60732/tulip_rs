use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "tweezerstop",
        full_name: "Tweezers Top",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Kenukitenjo",
    }
}

#[pattern_template(
    name = "TweezersTop",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(candle_type = "!Doji(FourPriceDoji) !Marubozu(WhiteMarubozu | ClosingWhiteMarubozu)"),
    bar(
        candle_type = "!Doji(FourPriceDoji) !Marubozu(WhiteMarubozu | ClosingWhiteMarubozu)",
        inside_prev = "LINE",
    ),
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, high, _, _) = inputs;

    high[FIRST] == high[SECOND]
}
