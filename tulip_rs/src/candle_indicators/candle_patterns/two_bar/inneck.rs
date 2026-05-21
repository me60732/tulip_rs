use crate::candle_indicators::{
    common::{cdl_height, cdl_real_in_body_position, cdl_wick_length},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "inneck",
        full_name: "In Neck",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Irikubi",
    }
}
#[pattern_template(
    name = "InNeck",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        lower_wick_2x = "FALSE",
        upper_wick_2x = "FALSE",
        open_in_prev_body = "FALSE",
        open_above_prev_mid = "FALSE",
        close_in_prev_body = "TRUE",
        close_above_prev_mid = "FALSE",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    let pos = cdl_real_in_body_position((open[FIRST], close[FIRST]), close[SECOND]);
    if pos > 15.0 {
        return false;
    }

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;

    if (bars[FIRST].lazy_computed & (1u16 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        bars[FIRST].set_body_height(cdl_height((open[FIRST], close[FIRST]), state.ema_body));
    }

    // SECOND bar: wick 2× bits — may already be pre-stamped false in CandleBits::new
    // when the corresponding wick_lt_body mandatory bit is set.
    let needs_lower_wick_2x =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT)) == 0;
    let needs_upper_wick_2x =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT)) == 0;
    if needs_lower_wick_2x || needs_upper_wick_2x {
        if needs_lower_wick_2x {
            bars[SECOND].set_lower_wick_2x(cdl_wick_length(
                (open[SECOND], close[SECOND]),
                low[SECOND],
                Some(2.0),
            ));
        }
        if needs_upper_wick_2x {
            bars[SECOND].set_upper_wick_2x(cdl_wick_length(
                (open[SECOND], close[SECOND]),
                high[SECOND],
                Some(2.0),
            ));
        }
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
