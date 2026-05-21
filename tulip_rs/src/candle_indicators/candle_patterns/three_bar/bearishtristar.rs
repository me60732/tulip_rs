//! Bearish Abandoned Baby (Sute go) - Three Bar Bearish Reversal Pattern

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishtristar",
        full_name: "Bearish Tri-Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Santen boshi",
    }
}

#[pattern_template(
    name = "BearishTriStar",
    forecast = "BearishReversal",
    prev_bar (trend = "UP"),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",

    ),
    bar(
        colour = "RED",
        body_gap = "GAP_DOWN"
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    true
}
