use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "invertedhammer",
        full_name: "Inverted Hammer",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Tohba",
    }
}

#[pattern_template(
    name = "InvertedHammer",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        body_height = "SHORT",
        lower_wick_lt_body = "TRUE",
        upper_wick_2x = "TRUE",
        open_above_prev_mid = "FALSE",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    if open[SECOND] > close[FIRST] {
        return false;
    }

    true
}
