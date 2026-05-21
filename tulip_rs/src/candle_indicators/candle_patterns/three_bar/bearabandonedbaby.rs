//! Bearish Abandoned Baby (Sute go) - Three Bar Bearish Reversal Pattern

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::{cdl_height, cdl_real_in_body_position, cdl_real_within_body, SHORT},
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearabandonedbaby",
        full_name: "Bearish Abandoned Baby",
        forcast: ForcastType::BearishReversal,
        bars: 3,
        extended_pattern: None,
        japanese_name: "Sute go",
    }
}

#[pattern_template(
    name = "BearishAbandonedBaby",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_UP"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
        wick_gap = "GAP_DOWN"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;
    println!("BearAbandonedBaby");
    if cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD])
        || cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND])
        || cdl_real_within_body((open[FIRST], close[FIRST]), close[SECOND])
    {
        return false;
    }

    if cdl_real_in_body_position((open[FIRST], close[FIRST]), close[THIRD]) > 50.0 {
        return false;
    }

    if cdl_height((high[THIRD], low[THIRD]), state.ema_line) == SHORT
        || cdl_height((high[FIRST], low[FIRST]), state.ema_line) == SHORT
    {
        return false;
    }

    true
}
