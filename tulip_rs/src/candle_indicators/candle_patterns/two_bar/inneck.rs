use crate::candle_indicators::{
    common::cdl_real_in_body_position,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "inneck",
        full_name: "In Neck",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Irikubi",
    }
}
#[pattern_template(
    name = "InNeck",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        lower_wick_2x = "FALSE",
        upper_wick_2x = "FALSE",
        open_in_prev_body = "FALSE",
        open_above_prev_mid = "FALSE",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    let pos = cdl_real_in_body_position((open[FIRST], close[FIRST]), close[SECOND]);
    if pos > 15.0 {
        return false;
    }

    true
}
