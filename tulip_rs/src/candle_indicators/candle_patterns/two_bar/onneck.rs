use crate::candle_indicators::{
    common::{cdl_height, cdl_wick_length},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "onneck",
        full_name: "On Neck",
        forcast: ForcastType::BearishContinuation,
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

    if close[SECOND] != low[FIRST] {
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

    // SECOND bar: apply_gap sets high_in_prev_line and all open/close position bits.
    let body_pos_mask =
        (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
    if (bars[SECOND].lazy_computed & body_pos_mask) != body_pos_mask {
        bars[SECOND].apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }

    // SECOND bar: wick 2× bits — may already be pre-stamped false in CandleBits::new
    // when the corresponding wick_lt_body mandatory bit is set.
    let needs_lower_wick_2x =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT)) == 0;
    let needs_upper_wick_2x =
        (bars[SECOND].lazy_computed & (1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT)) == 0;
    if needs_lower_wick_2x || needs_upper_wick_2x {
        if needs_lower_wick_2x {
            bars[SECOND].set_lower_wick_2x(
                cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], Some(2.0)),
            );
        }
        if needs_upper_wick_2x {
            bars[SECOND].set_upper_wick_2x(
                cdl_wick_length((open[SECOND], close[SECOND]), high[SECOND], Some(2.0)),
            );
        }
    }
}
