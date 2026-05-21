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
        name: "downsidetasukigap",
        full_name: "Downside Tasuki Gap",
        forcast: ForcastType::BearishContinuation,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Shita banare tasuki",
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
    name = "DownsideTasukiGap",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        wick_gap = "GAP_DOWN",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
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

    if !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD]) {
        return false;
    }

    if !(close[THIRD] < close[FIRST] && close[THIRD] > open[SECOND]) {
        return false;
    }

    // All conditions met
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let bar = &mut bars[2];

    if (bar.lazy_computed & (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)) == 0 {
        bar.apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
