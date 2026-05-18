use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_height, cdl_wick_length, LONG, SHORT},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::FIRST;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "hangingman",
        full_name: "Hanging Man",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 1,
        japanese_name: "kubitsuri",
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
    name = "HangingMan",
    forecast = "BearishReversal",
    prev_bar(trend = "UP")
    bar(
        body_height = "SHORT",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;
    // Ensure body_height is computed (needed by pattern template filter)
    

    if cdl_wick_length((open[FIRST], close[FIRST]), low[FIRST], Some(2.0)) == SHORT {
        return false;
    }
    if cdl_wick_length((open[FIRST], close[FIRST]), high[FIRST], None) == LONG {
        return false;
    }

    if !(state.ema < close[FIRST] && state.ema < open[FIRST]) {
        return false;
    }
    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;
    let current_bar = &mut bars[1];
    // Ensure body_height is computed (needed by pattern template filter)
    if (current_bar.computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        current_bar.set_body_height(body_height);
    }
}