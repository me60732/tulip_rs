use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_total_range,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearsidebysidewhitelines",
        full_name: "Bearish Side by Side White Lines",
        forcast: ForcastType::BearishContinuation,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Narabi aka",
    }
}

#[pattern_template(
    name = "BearSideBySideWhiteLines",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "HALLOW",
        wick_gap = "GAP_DOWN",
        candle_type = "Basic(ShortWhiteCandle | WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        fill = "HALLOW",
        candle_type = "Basic(ShortWhiteCandle | WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Basic pattern matching is already done by registry:
    // - Trend is uptrend
    // - 3 bars present
    // - All bars are GREEN and HALLOW
    // - Bar 1 matches required candle types
    //
    // This function ONLY checks relational constraints between bars

    // Safety: need at least 2 bars before current (i-2 must be valid)
    let (open, high, low, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===

    //body can not be greater then the first body
    let first_line_body = cdl_total_range(open[FIRST], close[FIRST]);
    if cdl_total_range(open[SECOND], close[SECOND]) > first_line_body {
        return false;
    }
    if cdl_total_range(open[THIRD], close[THIRD]) > first_line_body {
        return false;
    }

    //third must wick gap bellow first, second bar is in bar pattern wick_gap test already
    if !(low[FIRST] > high[THIRD]) {
        return false;
    }

    //opening closing price should be simular
    let tollerance = state.ema_line * 0.05;
    if tollerance < (close[SECOND] - close[THIRD]).abs() {
        return false;
    }
    if tollerance < (open[SECOND] - open[THIRD]).abs() {
        return false;
    }

    true
}
