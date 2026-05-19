use crate::candle_indicators::{
    common::{cdl_real_within_body, cdl_gap},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use crate::candle_indicators::registry::CandleBits;
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "upsidetasukigap",
        full_name: "Upside Tasuki Gap",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Uwa banare tasuki",
    }
}

#[pattern_template(
    name = "UpsideTasukiGap",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar( 
        colour = "GREEN"
        fill = "HALLOW", 
        line_height = "LONG",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        wick_gap = "GAP_UP",
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits]
) -> bool {
    // Basic pattern matching is already done by registry:
    // - Trend is uptrend
    // - 3 bars present
    // - All bars are GREEN and HALLOW
    // - Bar 1 matches required candle types
    //
    // This function ONLY checks relational constraints between bars

    let (open, _, _, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===

    if !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD]) { return false }

    if !(close[THIRD] > close[FIRST]) {
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
    let (_, high, low, _) = inputs;
    let second_bar = &mut bars[2];
    
    if (second_bar.lazy_computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[FIRST], low[FIRST]), (high[SECOND], low[SECOND]));
        second_bar.set_wick_gap(gap);
    }
}
