///Southern Doji
/// Construction:
///    First candle
///     a candle in an uptrend
///     white body
///    Second candle
///     a doji candle
///     a body above the first candle's body
use crate::candle_indicators::{
    common::{cdl_height, cdl_wick_length, LONG, SHORT, cdl_gap},
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "twocandleshootingstar",
        full_name: "Two Candle Shooting Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Nagare Boshi",
    }
}

#[pattern_template(
    name = "TwoCandleShootingStar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        line_height = "LONG",
        candle_type = "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu | WhiteMarubozu)"
    ),
    bar(
        colour = "GREEN",
        body_height = "SHORT",
        body_gap = "GAP_UP",
        candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop | HighWave)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;
    
    if cdl_wick_length((open[SECOND], close[SECOND]), high[SECOND], Some(2.5)) == SHORT {
        return false;
    }
    if cdl_wick_length((open[SECOND], close[SECOND]), low[SECOND], None) == LONG {
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

    if (second_bar.lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[FIRST], close[FIRST]), (open[SECOND], close[SECOND]));
        second_bar.set_body_gap(gap);
    }
}
