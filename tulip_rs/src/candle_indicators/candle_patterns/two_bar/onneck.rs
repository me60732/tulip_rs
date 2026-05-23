use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "onneck",
        full_name: "On Neck",
        forecast: ForecastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Atekubi",
    }
}
#[pattern_template(
    name = "OnNeck",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        body_gap = "GAP_DOWN",
        colour = "RED",
        fill = "HALLOW",
        lower_wick_2x = "FALSE",
        upper_wick_2x = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, low, close) = inputs;

    close[SECOND] == low[FIRST]
}
