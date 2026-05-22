//! Advance Block (Sakizumari) - Three Bar Bearish Reversal Pattern
//!
//! A bearish reversal pattern that occurs in an uptrend, consisting of three
//! consecutive white (bullish) candles with progressively smaller bodies and
//! increasing upper shadows.

use crate::candle_indicators::{
    common::{cdl_body_greater_body, cdl_total_wick_length},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "advanceblock",
        full_name: "Advance Block",
        forcast: ForcastType::BearishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Sakizumari",
    }
}
/// Advanced pattern calculation with additional constraints
///
/// This performs checks beyond the basic pattern matching:
/// - Each bar opens within the previous bar's body
/// - Bodies get progressively smaller
/// - Upper shadows get progressively longer
///
/// The registry handles basic filtering (trend, bar count, colours, fills).
/// This function handles the complex relational checks between bars.
///
/// # Arguments
/// * `open` - Open prices array
/// * `high` - High prices array
/// * `low` - Low prices array
/// * `close` - Close prices array
/// * `i` - Current index in the arrays
/// * `_state` - EMA state for volatility calculations (unused here)
///
/// # Returns
/// `true` if all advanced conditions are met, `false` otherwise
#[pattern_template(
    name = "AdvanceBlock",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN", 
        fill = "HALLOW",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "TRUE"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "TRUE"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;


    // 2. Shadows of the second and third line should be longer than the first line
    let first_wick = cdl_total_wick_length(open[FIRST], high[FIRST], low[FIRST], close[FIRST]);

    if !(first_wick < cdl_total_wick_length(open[SECOND], high[SECOND], low[SECOND], close[SECOND]))
        || !(first_wick < cdl_total_wick_length(open[THIRD], high[THIRD], low[THIRD], close[THIRD]))
    {
        return false;
    }

    // 3. Body height in each bar must be smaller than the previous bar
    if cdl_body_greater_body(
        (open[THIRD], close[THIRD]),
        (open[SECOND], close[SECOND]),
        1.0,
    ) || cdl_body_greater_body(
        (open[SECOND], close[SECOND]),
        (open[FIRST], close[FIRST]),
        1.0,
    ) {
        return false;
    }

    // All conditions met
    true
}
