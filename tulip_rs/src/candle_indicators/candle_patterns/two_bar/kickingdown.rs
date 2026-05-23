use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "kickingdown",
        full_name: "Kicking Down",
        forecast: ForecastType::BearishReversalOrContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Keri Ashi",
    }
}

#[pattern_template(
    name = "KickingDown",
    forecast = "BearishReversalOrContinuation",
    bar(
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Marubozu(WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        wick_gap = "GAP_DOWN",
        body_height = "LONG"
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Marubozu(BlackMarubozu)"
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {


    true
}
