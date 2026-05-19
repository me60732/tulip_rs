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
        name: "twocrows",
        full_name: "Two Crows",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Niwa Garasu",
    }
}

#[pattern_template(
    name = "TwoCrows",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN", 
        fill = "HALLOW", 
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        body_gap = "GAP_UP"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
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

    if !cdl_real_within_body((open[FIRST], close[FIRST]), close[THIRD]) 
    && !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD]) 
    { return false }
    
    // All conditions met
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;
    let second_bar = &mut bars[SECOND];

    if (second_bar.lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND]));
        second_bar.set_body_gap(gap);
    }
}
