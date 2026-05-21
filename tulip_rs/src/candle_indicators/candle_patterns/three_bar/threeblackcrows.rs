use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_real_within_body,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeblackcrows",
        full_name: "Three Black Crows",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Doji sanba garasu",
    }
}

#[pattern_template(
    name = "ThreeBlackCrows",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Basic pattern matching is already done by registry:
    // - Trend is uptrend
    // - 3 bars present
    // - All bars are GREEN and HALLOW
    // - Bar 1 matches required candle types
    //
    // This function ONLY checks relational constraints between bars

    let (open, _, _, close) = inputs;

    if !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD])
        || !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
    {
        return false;
    }

    // All conditions met
    true
}
