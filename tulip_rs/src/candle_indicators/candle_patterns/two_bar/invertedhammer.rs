use crate::candle_indicators::{
    common::{cdl_height, cdl_wick_length, SHORT},
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
        colour = "RED",
        fill = "FILL",
        line_height = "LONG",
        candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
    ),
    bar(
        body_height = "SHORT",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    if close[SECOND] == low[FIRST] || open[SECOND] > close[FIRST] {
        return false;
    }

    // Fast bit check: if current bar is a HighWave, require longer upper wick
    if bars[SECOND].mandatory & CandleBits::HIGH_WAVE != 0 {
        if cdl_wick_length((open[SECOND], close[SECOND]), high[SECOND], Some(2.5)) == SHORT {
            return false;
        }
    }

    // Lower wick must be short
    if cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], None) != SHORT {
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

    let second_bar = &mut bars[SECOND];
    
    if (second_bar.lazy_computed & (1 << CandleBits::BODY_HEIGHT_BIT)) == 0 {
        let body_height = cdl_height((open[SECOND], close[SECOND]), state.ema_body);
        second_bar.set_body_height(body_height);
    }
}
