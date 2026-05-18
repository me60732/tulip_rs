use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::{cdl_real_in_body_position, cdl_wick_length, LONG}
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "inneck",
        full_name: "In Neck",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Irikubi",
    }
}
#[pattern_template(
    name = "InNeck",
    forecast = "BearishContinuation",
    prev_bar (trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
    bar(
        colour = "GREEN", 
        fill = "HALLOW",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    let (open, high, low, close) = inputs;
    
    if !(open[SECOND] < close[FIRST]) { return false }

    if cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], Some(2.0000001)) == LONG
    || cdl_wick_length((open[SECOND], close[SECOND]), high[SECOND], Some(2.0000001)) == LONG 
    { return false }
    
    let pos = cdl_real_in_body_position((open[FIRST], close[FIRST]), close[SECOND]);
    if !(pos > 0.0 && pos < 15.0) { return false }
    
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
