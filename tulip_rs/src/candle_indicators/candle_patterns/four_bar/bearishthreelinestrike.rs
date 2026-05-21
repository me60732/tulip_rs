use super::{FIRST, FOURTH, SECOND, THIRD};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_bar_engulf_bar, cdl_real_within_body},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishthreelinestrike",
        full_name: "Bearish Three Line Strike",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 4,
        japanese_name: "Santeuchi",
    }
}

#[pattern_template(
    name = "BearishThreeLineStrike",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    // === Additional Constraints Beyond Basic Pattern Match ===
    if !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
        || !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD])
    {
        return false;
    }

    if !cdl_bar_engulf_bar((open[FOURTH], close[FOURTH]), (open[FIRST], close[THIRD])) {
        return false;
    }
    // All conditions met
    true
}
