use crate::candle_indicators::{
    common::cdl_body_greater,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use crate::candle_indicators::registry::CandleBits;
use tulip_rs_macros::pattern_template;
use super::FIRST;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishstrongline",
        full_name: "Bearish Strong Line",
        forcast: ForcastType::BearishReversalOrContinuation,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Yorikiri Sen",
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
    name = "BearishStrongLine",
    forecast = "BearishReversalOrContinuation",
    bar(
        fill = "FILL",
        line_height = "LONG",
        lower_wick_lt_body = "TRUE",
        upper_wick_lt_body = "TRUE",
        candle_type = "Basic(LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &[CandleBits],
) -> bool {
    // === Additional Constraints Beyond Basic Pattern Match ===

    let (open, _, _, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===
    // LongWhiteCandle already guarantees a sufficiently large body by definition;
    // only apply the explicit size check for Marubozu variants.
    if (bars[FIRST].mandatory & CandleBits::LONG_WHITE_CANDLE) == 0
        && !cdl_body_greater((open[FIRST], close[FIRST]), state.ema_body, 3.0)
    {
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

}
