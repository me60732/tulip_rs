//! Bearish Abandoned Baby (Sute go) - Three Bar Bearish Reversal Pattern

use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_gap,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "collapsingdojistar",
        full_name: "Collapsing Doji Star",
        forcast: ForcastType::BearishReversal,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Hōkai suru dōjī sutā",
    }
}

#[pattern_template(
    name = "Collapsingdojistar",
    forecast = "BearishReversal",
    prev_bar(trend = "UP"),
    bar(
        fill = "HALLOW",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    ),
    bar(
        colour = "RED",
        candle_type = "Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji)",
        wick_gap = "GAP_DOWN"
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
        wick_gap = "GAP_DOWN"
    )
)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (_, high, low, _) = inputs;
    
    if (bars[THIRD].computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[SECOND], low[SECOND]), (high[THIRD], low[THIRD]));
        bars[THIRD].set_wick_gap(gap);
    }
    
    if (bars[SECOND].computed & (1 << CandleBits::WICK_GAP_PRESENT_BIT)) == 0 {
        let gap = cdl_gap::<false>((high[FIRST], low[FIRST]), (high[SECOND], low[SECOND]));
        bars[SECOND].set_wick_gap(gap);
    }
}
