use crate::candle_indicators::types::{CandleInfo, ForcastType};
use tulip_rs_macros::pattern_template;

#[pattern_template(
    name = "TwoBlackGappingCandles",
    forecast = "BearishContinuation",
    prev_bar(trend = "DOWN"),
    bar(
        fill = "FILL",
        colour = "RED",
        body_gap = "GAP_DOWN",
        lower_wick_2x = "FALSE",
        upper_wick_2x = "FALSE",
        candle_type = "!Doji(FourPriceDoji | Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji) !SpinningTop(HighWave)"
    ),
    bar(
        fill = "FILL",
        colour = "RED",
        lower_wick_2x = "FALSE",
        upper_wick_2x = "FALSE",
        open_in_prev_body = "TRUE",
        close_in_prev_body = "FALSE",
        close_above_prev_mid = "FALSE",
        candle_type = "!Doji(FourPriceDoji | Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji) !SpinningTop(HighWave)"
    )
)]
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
