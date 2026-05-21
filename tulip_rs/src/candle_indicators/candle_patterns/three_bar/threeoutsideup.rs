//! Three Outside Up (Sanpei gaishi) - Three Bar Bullish Reversal Pattern
//!
//! A bullish reversal pattern that occurs in a downtrend.
//! It consists of a bearish candle, followed by a bullish engulfing candle,
//! and confirmed by a third bullish candle that closes above the second candle.

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeoutsideup",
        full_name: "Three Outside Up",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sanpei gaishi",
    }
}
#[pattern_template(
    name = "ThreeOutsideUp",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(colour = "RED", fill = "FILL", candle_type = "!Doji(FourPriceDoji)"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        engulf_prev = "BODY"
    ),
    bar(colour = "GREEN", fill = "HALLOW",)
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // SECOND engulfing FIRST's body is enforced by engulf_prev = "BODY".
    true
}
