use crate::candle_indicators::common::{
    BOTH_WICK, CandleShape, DOJI_MAX_HEIGHT, LONG, NO_BOTTOM_WICK, NO_TOP_WICK, NO_WICK, SHORT, cdl_body_position, cdl_total_range
};
use crate::candle_indicators::{pattern_test::EmaState as State, types::CandleStick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CDLDoji {
    Doji, //body located anywhere but the top/bottom/middle of the line
    LongLeggedDoji,
    DragonflyDoji,
    GravestoneDoji,
    FourPriceDoji,
}

impl CandleStick for CDLDoji {
    type Classified = CDLDoji;
    fn to_string(&self) -> String {
        match self {
            CDLDoji::Doji => "Doji".to_string(),
            CDLDoji::LongLeggedDoji => "Long Legged Doji".to_string(),
            CDLDoji::DragonflyDoji => "Dragonfly Doji".to_string(),
            CDLDoji::GravestoneDoji => "Gravestone Doji".to_string(),
            CDLDoji::FourPriceDoji => "Four Price Doji".to_string(),
        }
    }

    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, state: &State) -> bool {
        CDLDoji::is_candlestick_fast(open, high, low, close, &mut CandleShape::default(), state)
    }
    #[inline(always)]
    fn is_candlestick_fast(open: f64, _: f64, _: f64, close: f64, _candle_shape: &mut CandleShape, state: &State) -> bool {
        let body_range = cdl_total_range(open, close);

        if body_range <= state.ema_body * DOJI_MAX_HEIGHT {
            return true;
        }
        false
    }

    fn classify(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        state: &State,
    ) -> Option<Self::Classified> {
        CDLDoji::classify_fast(open, high, low, close, &mut CandleShape::default(), state)
    }
    #[inline(always)]
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        state: &State,
    ) -> Option<Self::Classified> {
        if !CDLDoji::is_candlestick_fast(open, high, low, close, candle_shape, state) {
            return None;
        }
        if candle_shape.line_height == LONG {
            if let Some(body_position) = cdl_body_position(open, high, low, close) {
                if CDLDoji::is_long_legged_doji(body_position, candle_shape) {
                    return Some(CDLDoji::LongLeggedDoji);
                }
                // Then, check for Dragonfly Doji.
                if CDLDoji::is_dragonfly_doji(body_position, candle_shape) {
                    return Some(CDLDoji::DragonflyDoji);
                }
                // Then, check for Gravestone Doji.
                if CDLDoji::is_gravestone_doji(body_position, candle_shape) {
                    return Some(CDLDoji::GravestoneDoji);
                }
            }
        } else {
            if CDLDoji::is_four_price_doji(open, high, low, close, candle_shape) {
                // Additional condition example: sometimes you want to check that the body is near zero.
                //if (open - close).abs() < 0.05 { //////// Dont know why this is here
                return Some(CDLDoji::FourPriceDoji);
            }
        }
        // Finally, if it's a doji (based on body size relative to total range), choose the basic Doji.
        return Some(CDLDoji::Doji);
    }
    #[inline(always)]
    fn discriminant(&self) -> u8 {
        match self {
            CDLDoji::Doji => 0,
            CDLDoji::LongLeggedDoji => 1,
            CDLDoji::DragonflyDoji => 2,
            CDLDoji::GravestoneDoji => 3,
            CDLDoji::FourPriceDoji => 4,
        }
    }
    #[inline(always)]
    fn to_bit(&self) -> u8 {
        match self {
            CDLDoji::Doji => 1 << 0,           // Bit 0
            CDLDoji::LongLeggedDoji => 1 << 1, // Bit 1
            CDLDoji::DragonflyDoji => 1 << 2,  // Bit 2
            CDLDoji::GravestoneDoji => 1 << 3, // Bit 3
            CDLDoji::FourPriceDoji => 1 << 4,  // Bit 4
        }
    }
    #[inline(always)]
    fn from_bit(bits: u8) -> Option<Self::Classified> {
        match bits {
            b if b & (1 << 0) != 0 => Some(CDLDoji::Doji),
            b if b & (1 << 1) != 0 => Some(CDLDoji::LongLeggedDoji),
            b if b & (1 << 2) != 0 => Some(CDLDoji::DragonflyDoji),
            b if b & (1 << 3) != 0 => Some(CDLDoji::GravestoneDoji),
            b if b & (1 << 4) != 0 => Some(CDLDoji::FourPriceDoji),
            _ => None,
        }
    }
}

impl CDLDoji {
    #[inline(always)]
    fn is_long_legged_doji(body_position: f64, candle_shape: &mut CandleShape) -> bool {
        if body_position > 45.0 && body_position < 55.0 {
            candle_shape.wick = Some(BOTH_WICK);
            candle_shape.top_wick_length = Some(LONG);
            candle_shape.bottom_wick_length = Some(LONG);
            return true;
        }

        false
    }
    #[inline(always)]
    fn is_dragonfly_doji(body_position: f64, candle_shape: &mut CandleShape) -> bool {
        if body_position == 100.0 {
            candle_shape.wick = Some(NO_BOTTOM_WICK);
            candle_shape.top_wick_length = Some(SHORT);
            candle_shape.bottom_wick_length = Some(LONG);
            return true;
        }

        false
    }
    #[inline(always)]
    fn is_gravestone_doji(body_position: f64, candle_shape: &mut CandleShape) -> bool {
        if body_position == 0.0 {
            candle_shape.wick = Some(NO_TOP_WICK);
            candle_shape.top_wick_length = Some(LONG);
            candle_shape.bottom_wick_length = Some(SHORT);
            return true;
        }
        false
    }
    #[inline(always)]
    fn is_four_price_doji(open: f64, high: f64, low: f64, close: f64, candle_shape: &mut CandleShape) -> bool {
        if candle_shape.get_wick(open, high, low, close) == NO_WICK {
            candle_shape.top_wick_length = Some(SHORT);
            candle_shape.bottom_wick_length = Some(SHORT);
            return true;
        }

        false
    }
}
