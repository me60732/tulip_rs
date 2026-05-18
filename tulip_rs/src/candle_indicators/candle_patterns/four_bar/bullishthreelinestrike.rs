use crate::candle_indicators::{
    common::{cdl_real_within_body, cdl_bar_engulf_bar},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use crate::candle_indicators::registry::CandleBits;
use tulip_rs_macros::pattern_template;
use super::{FIRST, SECOND, THIRD, FOURTH};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishthreelinestrike",
        full_name: "Bullish Three Line Strike",
        forcast: ForcastType::BullishContinuation,
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
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(FourPriceDoji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Doji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
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

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &mut [CandleBits],
) {
    // No lazy bits needed for this pattern
}
