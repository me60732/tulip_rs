use super::THIRD;
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "concealingbabyswallow",
        full_name: "Concealing Baby Swallow",
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,
        bars: 4,
        japanese_name: "kotsubame tsutsumi",
    }
}

#[pattern_template(
    name = "ConcealingBabySwallow",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED", 
        candle_type = "Marubozu(BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        open_in_prev_body = "TRUE",
        body_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN",
        high_in_prev_body = "TRUE",
        lower_wick_lt_body = "TRUE",
        candle_type = "SpinningTop(HighWave)"
    ),
    bar(
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        engulf_prev = "LINE"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, low, close) = inputs;
    low[THIRD] == close[THIRD] 
}
