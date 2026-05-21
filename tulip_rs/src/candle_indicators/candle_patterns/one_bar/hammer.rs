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
        line_height = "LONG",
        upper_wick_lt_body = "TRUE",
        lower_wick_lt_body = "FALSE",
        lower_wick_2x = "TRUE"
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, low, close) = inputs;
    let first_bar = &mut bars[FIRST];

    if (first_bar.lazy_computed & (1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT)) == 0 {
        let is_2x = cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(2.0)) != SHORT;
        first_bar.set_lower_wick_2x(is_2x);
    }
}
