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
        name: "invertedhammer",
        full_name: "Inverted Hammer",
        forcast: ForcastType::BullishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Tohba",
    }
}

#[pattern_template(
    name = "InvertedHammer",
    forecast = "BullishReversal",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        line_height = "LONG",
        body_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        body_height = "SHORT",
        lower_wick_lt_body = "TRUE",
        upper_wick_2x = "TRUE",
        open_above_prev_mid = "FALSE",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;

    if open[SECOND] > close[FIRST] {
        return false;
    }

    true
}

pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, _, close) = inputs;
    let height_mask = 1u16 << CandleBits::BODY_HEIGHT_BIT;
    if (bars[FIRST].lazy_computed & height_mask) == 0 {
        bars[FIRST].set_body_height(cdl_height((open[FIRST], close[FIRST]), state.ema_body));
    }
    if (bars[SECOND].lazy_computed & height_mask) == 0 {
        bars[SECOND].set_body_height(cdl_height((open[SECOND], close[SECOND]), state.ema_body));
    }

    if (bars[SECOND].lazy_computed & (1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT)) == 0 {
        bars[SECOND].set_upper_wick_2x(cdl_wick_length(
            (open[SECOND], close[SECOND]),
            high[SECOND],
            Some(2.0),
        ));
    }

    if (bars[SECOND].lazy_computed & (1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT)) == 0 {
        let body_mid = (open[FIRST].max(close[FIRST]) + open[FIRST].min(close[FIRST])) / 2.0;
        bars[SECOND].set_open_above_mid(open[SECOND] > body_mid);
    }
}
