use crate::candle_indicators::{
    pattern_test::EmaState,
    registry::CandleBits,
    types::{CandleInfo, ForcastType},
    common::cdl_gap
};
use tulip_rs_macros::pattern_template;

use super::{PREV, FIRST, SECOND};


pub fn info() -> CandleInfo {
    CandleInfo {
        name: "twoblackgappingcandles",
        full_name: "Two Black Gapping Candles",
        forcast: ForcastType::BearishContinuation,
        extended_pattern: None,
        bars: 2,
        japanese_name: "Nihon no kuroi madoake rōsoku ashi",
    }
}

#[pattern_template(
    name = "TwoBlackGappingCandles",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        colour = "RED",
        body_gap = "GAP_DOWN",
        candle_type = "!Doji(FourPriceDoji | Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji) !SpinningTop(HighWave)"
    ),
    bar(
        fill = "FILL",
        colour = "RED",
        candle_type = "!Doji(FourPriceDoji | Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji) !SpinningTop(HighWave)"
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &[CandleBits],
) -> bool {
    let (open, high, low, close) = inputs;

    if !(open[FIRST] > open[SECOND] && open[SECOND] > low[FIRST]) {
        return false;
    }

    // Check if either bar is a BlackSpinningTop and validate wick lengths
    // bars[0] is prev_bar, bars[1] is first pattern bar, bars[2] is second pattern bar
    let first_bar = bars[FIRST];
    let second_bar = bars[SECOND];

    // Check first bar - use bit masking to detect BlackSpinningTop
    if (first_bar.mandatory & CandleBits::BLACK_SPINNING_TOP) != 0 {
        let body_length = (open[FIRST] - close[FIRST]).abs();
        let top_wick = high[FIRST] - open[FIRST];
        let bottom_wick = close[FIRST] - low[FIRST];

        if top_wick > 2.0 * body_length || bottom_wick > 2.0 * body_length {
            return false;
        }
    }

    // Check second bar - use bit masking to detect BlackSpinningTop
    if (second_bar.mandatory & CandleBits::BLACK_SPINNING_TOP) != 0 {
        let body_length = (open[SECOND] - close[SECOND]).abs();
        let top_wick = high[SECOND] - open[SECOND];
        let bottom_wick = close[SECOND] - low[SECOND];

        if top_wick > 2.0 * body_length || bottom_wick > 2.0 * body_length {
            return false;
        }
    }

    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, _, _, close) = inputs;
    let first_bar = &mut bars[FIRST];

    if (first_bar.lazy_computed & (1 << CandleBits::BODY_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<true>((open[PREV], close[PREV]), (open[FIRST], close[FIRST]));
        first_bar.set_body_gap(gap);
    }
}
