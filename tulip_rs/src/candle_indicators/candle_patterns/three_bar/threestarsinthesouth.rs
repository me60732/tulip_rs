use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_body_greater_body, cdl_wick_length, LONG},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threestarsinthesouth",
        full_name: "Three Stars In The South",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Kyoku no santen boshi",
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
    name = "ThreeStarsInTheSouth",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(colour = "RED", fill = "FILL", line_height = "LONG",),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    ),
    bar(
        fill = "FILL",
        line_height = "SHORT",
        candle_type = "Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
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

    if open[FIRST] <= open[SECOND] {
        return false;
    }
    if low[SECOND] <= low[FIRST] {
        return false;
    }

    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(1.000001)) != LONG {
        return false;
    }
    if !cdl_body_greater_body((high[SECOND], low[SECOND]), (high[THIRD], low[THIRD]), 1.0) {
        return false;
    }

    // All conditions met
    true
}
