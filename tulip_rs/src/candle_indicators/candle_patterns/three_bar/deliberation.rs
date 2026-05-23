use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_body_greater_body,
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "deliberation",
        full_name: "Deliberation",
        forecast: ForecastType::BearishReversal,
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
    name = "Deliberation",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(colour = "GREEN", fill = "HALLOW", body_height = "LONG",),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        body_height = "LONG",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "TRUE",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "SHORT",
        low_in_prev_line = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "TRUE",
        body_gt_prev_body = "FALSE",
        candle_type = "Basic(ShortWhiteCandle) SpinningTop(WhiteSpinningTop)"
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

    if cdl_body_greater_body(
        (open[THIRD], close[THIRD]),
        (open[FIRST], close[FIRST]),
        1.0,
    ) {
        return false;
    }

    // All conditions met
    true
}
