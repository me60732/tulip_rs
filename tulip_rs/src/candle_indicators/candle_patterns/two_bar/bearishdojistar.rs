///Southern Doji
/// Construction:
///    First candle
///     a candle in an uptrend
///     white body
///    Second candle
///     a doji candle
///     a body above the first candle's body
use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_gap
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishdojistar",
        full_name: "Bearish Doji Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Doji Bike",
    }
}

#[pattern_template(
    name = "BearishDojiStar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "HALLOW"
        colour = "GREEN",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        body_gap = "GAP_UP"
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    
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
