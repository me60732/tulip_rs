use super::{FIRST, PREV};
use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "gapupdoji",
        full_name: "Gapping Up Doji",
        extended_pattern: None,
        forcast: ForcastType::BullishContinuation,
        bars: 1,
        japanese_name: "Ue-hanare Doji",
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
    name = "GappingUpDoji",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_UP"
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
    let (open, high, low, close) = inputs;
    let current_bar = &mut bars[1];

    if (current_bar.lazy_computed & (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT)) == 0 {
        current_bar.apply_gap(
            (open[PREV], high[PREV], low[PREV], close[PREV]),
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
        );
    }
}
