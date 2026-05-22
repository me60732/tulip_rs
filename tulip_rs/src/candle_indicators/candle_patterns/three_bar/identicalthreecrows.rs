use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_real_in_body_position,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "identicalthreecrows",
        full_name: "Identical Three Crows",
        forcast: ForcastType::BearishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Doji sanba garasu",
    }
}

#[pattern_template(
    name = "IdenticalThreeCrows",
    forecast = "BearishReversal",
    prev_bar (trend = "UP"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;
    let second_pos = cdl_real_in_body_position((open[FIRST], close[FIRST]), open[SECOND]);
    let third_pos = cdl_real_in_body_position((open[SECOND], close[SECOND]), open[THIRD]);
    if !(-5.0..=5.0).contains(&second_pos) {
        return false;
    }
    if !(-5.0..=5.0).contains(&third_pos) {
        return false;
    }

    true
}
