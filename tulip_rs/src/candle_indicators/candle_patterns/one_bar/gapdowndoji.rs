use super::{FIRST, PREV};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_gap,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "gapdowndoji",
        full_name: "Gapping Down Doji",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 1,
        japanese_name: "Shita-hanare Doji",
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
    name = "GappingDownDoji",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_DOWN"
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
    let (_, high, low, _) = inputs;
    let current_bar = &mut bars[1];

    if (current_bar.lazy_computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[PREV], low[PREV]), (high[FIRST], low[FIRST]));
        current_bar.set_wick_gap(gap);
    }
}
