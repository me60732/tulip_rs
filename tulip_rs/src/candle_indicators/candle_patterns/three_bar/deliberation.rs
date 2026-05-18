use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_body_greater_body, cdl_height, cdl_total_wick_length},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "deliberation",
        full_name: "Deliberation",
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
    name = "Deliberation",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(colour = "GREEN", fill = "HALLOW", body_height = "LONG",),
    bar(colour = "GREEN", fill = "HALLOW", body_height = "LONG"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "SHORT",
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

    let (open, high, low, close) = inputs;

    //may not be neccusary
    if cdl_body_greater_body(
        (open[THIRD], close[THIRD]),
        (open[SECOND], close[SECOND]),
        1.0,
    ) {
        return false;
    }
    if cdl_body_greater_body(
        (open[THIRD], close[THIRD]),
        (open[FIRST], close[FIRST]),
        1.0,
    ) {
        return false;
    }

    if !(open[THIRD] > open[SECOND]) || open[THIRD] > close[SECOND] {
        return false;
    }
    if !(open[SECOND] > open[FIRST]) || open[SECOND] > close[FIRST] {
        return false;
    }

    let first_wick = cdl_total_wick_length(open[FIRST], high[FIRST], low[FIRST], close[FIRST]);

    if !(first_wick > cdl_total_wick_length(open[SECOND], high[SECOND], low[SECOND], close[SECOND]))
        || !(first_wick > cdl_total_wick_length(open[THIRD], high[THIRD], low[THIRD], close[THIRD]))
    {
        return false;
    }

    // All conditions met
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;

    if (bars[2].computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[SECOND], close[SECOND]), state.ema_body);
        bars[2].set_body_height(body_height);
    }
    // Ensure 1st bar body_height is computed (needed by pattern template filter)
    if (bars[1].computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        bars[1].set_body_height(body_height);
    }
}
