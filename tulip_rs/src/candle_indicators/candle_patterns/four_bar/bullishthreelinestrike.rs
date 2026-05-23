use super::{FIRST, SECOND, FOURTH};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_bar_engulf_bar,
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishthreelinestrike",
        full_name: "Bullish Three Line Strike",
        forecast: ForecastType::BullishContinuation,
        extended_pattern: None,
        bars: 4,
        japanese_name: "Santeuchi",
    }
}

#[pattern_template(
    name = "BullishThreeLineStrike",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        open_in_prev_body = "TRUE",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        open_in_prev_body = "TRUE",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "RED",
        line_height = "LONG",
        body_height = "LONG",
        engulf_prev = "BODY",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    cdl_bar_engulf_bar((open[FOURTH], close[FOURTH]), (open[FIRST], close[SECOND]))
}
