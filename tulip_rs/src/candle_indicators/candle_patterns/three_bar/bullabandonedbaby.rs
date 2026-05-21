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
        name: "bullabandonedbaby",
        full_name: "Bullish Abandoned Baby",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Sute go",
    }
}

#[pattern_template(
    name = "BullAbandonedBaby",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        colour = "RED",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_DOWN"
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
        wick_gap = "GAP_UP"
    )
)]
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    if cdl_real_within_body((open[FIRST], close[FIRST]), open[SECOND]) {
        return false;
    }
    if cdl_real_within_body((open[FIRST], close[FIRST]), close[SECOND]) {
        return false;
    }
    if cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD]) {
        return false;
    }

    if cdl_real_in_body_position((open[FIRST], close[FIRST]), close[THIRD]) < 50.0 {
        return false;
    }

    if cdl_height((high[THIRD], low[THIRD]), state.ema_line) == SHORT {
        return false;
    }
    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;

    if (bars[3].lazy_computed & (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT)) == 0 {
        bars[3].apply_gap(
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
            (open[THIRD], high[THIRD], low[THIRD], close[THIRD]),
        );
    }

    if (bars[2].lazy_computed & (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)) == 0 {
        bars[2].apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
