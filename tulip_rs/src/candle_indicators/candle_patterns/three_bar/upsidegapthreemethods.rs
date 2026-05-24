use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_real_within_body,
    pattern_test::EmaState,
    types::{CandleInfo, ForecastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "upsidegapthreemethods",
        full_name: "Upside Gap Three Methods",
        forecast: ForecastType::BullishContinuation,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Uwa banare tasuki",
    }
}

#[pattern_template(
    name = "UpsideGapThreeMethods",
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
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "FALSE",
        candle_type = "Basic(BlackCandle | ShortBlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===

    cdl_real_within_body((open[FIRST], close[FIRST]), close[THIRD])

}
