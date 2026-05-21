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
        name: "threewhitesoldiers",
        full_name: "Three White Soldiers",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanpei",
    }
}

#[pattern_template(
    name = "ThreeWhiteSoldiers",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji) !SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji) !SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji) !SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
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

    // === Additional Constraints Beyond Basic Pattern Match ===

    if !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD])
        || !cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
    {
        return false;
    }

    // All conditions met
    true
}
