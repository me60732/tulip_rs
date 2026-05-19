use crate::candle_indicators::{
    common::cdl_gap,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "risingwindow",
        full_name: "Rising Window",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Shitakage",
    }
}
#[pattern_template(
    name = "RisingWindow",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(candle_type = "!Doji(FourPriceDoji)"),
    bar(
        wick_gap = "GAP_UP",
        colour = "GREEN",
        candle_type = "!Doji(FourPriceDoji)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    if !(close[FIRST] > state.ema) {
        return false;
    }

    true
}

/// Computes lazy bits required by this pattern's template.
///
/// Sets the wick gap bits on bar 2 (SECOND) so the registry's `matches_bars()`
/// can compare them against the `wick_gap = "GAP_UP"` mask declared in the template.
/// Does NOT check whether the gap is present — that is the registry's job.
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (_, high, low, _) = inputs;

    let second_bar = &mut bars[SECOND];
    // Compute wick gap for bar 2 — required by wick_gap = "GAP_UP" in the template
    if (second_bar.lazy_computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[FIRST], low[FIRST]), (high[SECOND], low[SECOND]));
        second_bar.set_wick_gap(gap);
    }
}
