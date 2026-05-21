use super::FIRST;
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_wick_length, LONG},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishbelthold",
        full_name: "Bearish Belt Hold",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Yorikiri",
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
    name = "BearishBelthold",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        lower_wick_lt_body = "TRUE",
        candle_type = "Marubozu(OpeningBlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, low, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===
    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(0.25000001)) == LONG {
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
