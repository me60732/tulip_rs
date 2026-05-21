use crate::candle_indicators::{
    common::{cdl_height, cdl_real_within_body},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, PREV, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "bullishtasukiline",
        full_name: "Bullish Tasuki Line",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Tasuki",
    }
}

#[pattern_template(
    name = "BullishTasukiLine",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        high_in_prev_line = "TRUE",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        body_height = "LONG",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "TRUE",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)",
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
    let (open, high, low, close) = inputs;

    if (bars[FIRST].lazy_computed & (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)) == 0 {
        bars[FIRST].set_high_in_line(high[FIRST] >= low[PREV] && high[FIRST] < high[PREV]);
    }

    if (bars[SECOND].lazy_computed & (1u16 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[SECOND], close[SECOND]), state.ema_body);
        bars[SECOND].set_body_height(body_height);
    }

    if (bars[SECOND].lazy_computed & (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)) == 0 {
        bars[SECOND].set_open_in_body(cdl_real_within_body(
            (open[FIRST], close[FIRST]),
            open[SECOND],
        ));
    }
    // Gate CLOSE_IN_PREV_BODY_BIT and CLOSE_ABOVE_PREV_BODY_MID_BIT independently —
    // the above-mid bit may already be set (e.g. by apply_gap), so only compute what's missing.
    let close_in_body_mask = 1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT;
    let close_above_mid_mask = 1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT;
    let needs_in_body = (bars[SECOND].lazy_computed & close_in_body_mask) == 0;
    let needs_above_mid = (bars[SECOND].lazy_computed & close_above_mid_mask) == 0;
    if needs_in_body || needs_above_mid {
        let body_top = open[FIRST].max(close[FIRST]);
        let body_bot = open[FIRST].min(close[FIRST]);
        if needs_in_body {
            bars[SECOND].set_close_in_body(close[SECOND] >= body_bot && close[SECOND] <= body_top);
        }
        if needs_above_mid {
            let body_mid = (body_top + body_bot) / 2.0;
            bars[SECOND].set_close_above_mid(close[SECOND] > body_mid);
        }
    }
}
