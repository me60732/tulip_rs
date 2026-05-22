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
        name: "bullsidebysidewhitelines",
        full_name: "Bullish Side by Side White Lines",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Uwappanare narabi aka",
    }
}

#[pattern_template(
    name = "BullSidebySideWhiteLines",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "Basic(ShortWhiteCandle | WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        wick_gap = "GAP_UP"
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
    let (open, high, low, close) = inputs;

    // === Additional Constraints Beyond Basic Pattern Match ===

    let first_line_body = cdl_total_range(open[FIRST], close[FIRST]);
    if cdl_total_range(open[SECOND], close[SECOND]) > first_line_body {
        return false;
    }
    if cdl_total_range(open[THIRD], close[THIRD]) > first_line_body {
        return false;
    }

    if !(high[FIRST] < low[THIRD]) {
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

    // All conditions met
    true
}
