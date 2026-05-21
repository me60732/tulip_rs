use crate::candle_indicators::{
    common::cdl_height,
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bearishmeetinglines",
        full_name: "Bearish Meeting Lines",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Deaisen",
    }
}

#[pattern_template(
    name = "BearishMeetingLines",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "FILL",
        body_height = "LONG",
        line_height = "LONG",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "TRUE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (_, _, _, close) = inputs;

    if !(close[FIRST] == close[SECOND]) {
        return false;
    }
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;
    let height_mask = 1u16 << CandleBits::BODY_HEIGHT_BIT;
    if (bars[FIRST].lazy_computed & height_mask) == 0 {
        let body_height = cdl_height((open[FIRST], close[FIRST]), state.ema_body);
        bars[FIRST].set_body_height(body_height);
    }
    if (bars[SECOND].lazy_computed & height_mask) == 0 {
        let body_height = cdl_height((open[SECOND], close[SECOND]), state.ema_body);
        bars[SECOND].set_body_height(body_height);
    }

    // SECOND bar: compute where SECOND's close sits within FIRST's body.
    // Gate each bit independently — CLOSE_ABOVE_PREV_BODY_MID_BIT may already be set
    // by apply_gap (e.g. close outside body is definitively above/below mid), so we
    // only recompute what is actually missing.
    let close_in_body_mask = 1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT;
    let close_above_mid_mask = 1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT;
    let needs_in_body = (bars[SECOND].lazy_computed & close_in_body_mask) == 0;
    let needs_above_mid = (bars[SECOND].lazy_computed & close_above_mid_mask) == 0;
    if needs_in_body || needs_above_mid {
        let body_top = open[FIRST].max(close[FIRST]);
        let body_bot = open[FIRST].min(close[FIRST]);
        if needs_in_body {
            bars[SECOND].set_close_in_body(close[SECOND] <= body_top && close[SECOND] >= body_bot);
        }
        if needs_above_mid {
            let body_mid = (body_top + body_bot) / 2.0;
            bars[SECOND].set_close_above_mid(close[SECOND] > body_mid);
        }
    }
}
