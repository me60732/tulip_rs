use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_wick_length, SHORT},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "hammer",
        full_name: "Hammer",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "kanazuchi",
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
    name = "Hammer",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN")
    bar(
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop)",
        line_height = "LONG"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
   _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    
    if cdl_wick_length((open[FIRST], close[FIRST]), high[FIRST], None) != SHORT {
        return false;
    }

    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(2.0)) == SHORT {
        return false;
    }
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
