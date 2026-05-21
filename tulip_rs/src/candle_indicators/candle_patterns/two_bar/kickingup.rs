use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "kickingup",
        full_name: "Kicking Up",
        forcast: ForcastType::BullishReversalOrContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Keri Ashi",
    }
}

#[pattern_template(
    name = "KickingUp",
    forecast = "BullishReversalOrContinuation",
    bar(
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    ),
    bar(
        colour = "GREEN",
        body_gap = "GAP_UP",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, _) = inputs;

    if !(open[FIRST] < open[SECOND]) {
        return false;
    }

    true
}
