use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "upsidetasukigap",
        full_name: "Upside Tasuki Gap",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Uwa banare tasuki",
    }
}

#[pattern_template(
    name = "UpsideTasukiGap",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN"
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        wick_gap = "GAP_UP",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle | ShortWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    // Basic pattern matching is already done by registry:
    // - Trend is uptrend
    // - 3 bars present
    // - All bars are GREEN and HALLOW
    // - Bar 1 matches required candle types
    //
    // This function ONLY checks relational constraints between bars

    let (_, _, _, close) = inputs;

    close[THIRD] > close[FIRST]
}
