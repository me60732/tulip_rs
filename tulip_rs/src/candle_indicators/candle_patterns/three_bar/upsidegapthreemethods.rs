use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::{
    common::cdl_real_within_body,
    pattern_test::EmaState,
    types::{CandleInfo, ForcastType},
};
use tulip_rs_macros::pattern_template;

use super::{FIRST, SECOND, THIRD};

pub fn info() -> CandleInfo {
    CandleInfo {
        name: "upsidegapthreemethods",
        full_name: "Upside Gap Three Methods",
        forcast: ForcastType::BullishContinuation,
        extended_pattern: None,
        bars: 3,
        japanese_name: "Uwa banare sanpoo hatsu oshi",
    }
}

#[pattern_template(
    name = "UpsideGapThreeMethods",
    forecast = "BullishContinuation",
    prev_bar(trend = "UP"),
    bar(
        colour = "GREEN"
        fill = "HALLOW",
        line_height = "LONG",
    ),
    bar(
        colour = "GREEN",
        fill = "HALLOW",
        wick_gap = "GAP_UP",
    ),
    bar(
        colour = "RED",
        fill = "FILL",
        candle_type = "!Doji(Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | FourPriceDoji)",
    )
)]

pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, _, _, close) = inputs;
    // === Additional Constraints Beyond Basic Pattern Match ===

    if !cdl_real_within_body((open[SECOND], close[SECOND]), open[THIRD])
        || !cdl_real_within_body((open[FIRST], close[FIRST]), close[THIRD])
    {
        return false;
    }

    // All conditions met
    true
}

/// Default compute_bits - this pattern doesn't use lazy bits
pub fn compute_bits(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    bars: &mut [CandleBits],
) {
    let (open, high, low, close) = inputs;
    let bar = &mut bars[2];

    if (bar.lazy_computed & (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT)) == 0 {
        bar.apply_gap(
            (open[FIRST], high[FIRST], low[FIRST], close[FIRST]),
            (open[SECOND], high[SECOND], low[SECOND], close[SECOND]),
        );
    }
}
