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
        name: "darkcloudcover",
        full_name: "Dark Cloud Cover",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Kumo no Ura",
    }
}
#[pattern_template(
    name = "DarkCloudCover",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "FALSE",
        open_above_prev_mid = "TRUE",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    )
)]

pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;

    let height_mask = 1u16 << CandleBits::BODY_HEIGHT_BIT;
    if (bars[FIRST].lazy_computed & height_mask) == 0 {
        bars[FIRST].set_body_height(cdl_height((open[FIRST], close[FIRST]), state.ema_body));
    }
    if (bars[SECOND].lazy_computed & height_mask) == 0 {
        bars[SECOND].set_body_height(cdl_height((open[SECOND], close[SECOND]), state.ema_body));
    }

    // Compute SECOND bar's open/close position relative to FIRST's body.
    // Gate each bit independently — bits may already be set (e.g. by apply_gap).
    let needs_open_in_body =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)) == 0;
    let needs_open_above_mid =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT)) == 0;
    let needs_close_in_body =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT)) == 0;
    let needs_close_above_mid =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT)) == 0;
    if needs_open_in_body || needs_open_above_mid || needs_close_in_body || needs_close_above_mid {
        let body_top = open[FIRST].max(close[FIRST]);
        let body_bot = open[FIRST].min(close[FIRST]);
        if needs_open_in_body {
            bars[SECOND].set_open_in_body(open[SECOND] >= body_bot && open[SECOND] <= body_top);
        }
        if needs_close_in_body {
            bars[SECOND].set_close_in_body(close[SECOND] >= body_bot && close[SECOND] <= body_top);
        }
        if needs_open_above_mid || needs_close_above_mid {
            let body_mid = (body_top + body_bot) / 2.0;
            if needs_open_above_mid {
                bars[SECOND].set_open_above_mid(open[SECOND] > body_mid);
            }
            if needs_close_above_mid {
                bars[SECOND].set_close_above_mid(close[SECOND] > body_mid);
            }
        }
    }
}
