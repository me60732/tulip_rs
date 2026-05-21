//! Three Outside Down (Sanpei gaishi) - Three Bar Bearish Reversal Pattern
//!
//! A bearish reversal pattern that occurs in an uptrend.
//! It consists of a bullish candle, followed by a bearish engulfing candle,
//! and confirmed by a third bearish candle that closes below the second candle.

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "threeoutsidedown",
        full_name: "Three Outside Down",
        forcast: ForcastType::BearishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Sanpei gaishi",
    }
}
#[pattern_template(
    name = "ThreeOutsideDown",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "!Doji(FourPriceDoji)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        engulf_prev = "BODY"
    ),
    bar(colour = "RED", fill = "FILL")
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // SECOND engulfing FIRST's body is enforced by engulf_prev = "BODY".
    true
}
