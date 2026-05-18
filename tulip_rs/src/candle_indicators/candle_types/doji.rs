use crate::candle_indicators::common::{
    cdl_body_position, cdl_height, cdl_no_wick, cdl_total_range, DOJI_MAX_HEIGHT, LONG, NO_WICK,
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
        CDLDoji::is_candlestick_fast(open, high, low, close, false, state)
    }
    #[inline(always)]
    fn is_candlestick_fast(open: f64, _: f64, _: f64, close: f64, _: bool, state: &State) -> bool {
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
        CDLDoji::classify_fast(open, high, low, close, false, state)
    }
    #[inline(always)]
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        _: bool,
        state: &State,
    ) -> Option<Self::Classified> {
        if !CDLDoji::is_candlestick_fast(open, high, low, close, false, state) {
            return None;
        }
        if cdl_height((high, low), state.ema_line) == LONG {
            if let Some(body_position) = cdl_body_position(open, high, low, close) {
                if CDLDoji::is_long_legged_doji(body_position) {
                    return Some(CDLDoji::LongLeggedDoji);
                }
                // Then, check for Dragonfly Doji.
                if CDLDoji::is_dragonfly_doji(body_position) {
                    return Some(CDLDoji::DragonflyDoji);
                }
                // Then, check for Gravestone Doji.
                if CDLDoji::is_gravestone_doji(body_position) {
                    return Some(CDLDoji::GravestoneDoji);
                }
            }
        } else {
            if CDLDoji::is_four_price_doji(open, high, low, close) {
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
    fn is_long_legged_doji(body_position: f64) -> bool {
        if body_position > 45.0 && body_position < 55.0 {
            return true;
        }

        false
    }
    #[inline(always)]
    fn is_dragonfly_doji(body_position: f64) -> bool {
        if body_position == 100.0 {
            return true;
        }

        false
    }
    #[inline(always)]
    fn is_gravestone_doji(body_position: f64) -> bool {
        if body_position == 0.0 {
            return true;
        }
        false
    }
    #[inline(always)]
    fn is_four_price_doji(open: f64, high: f64, low: f64, close: f64) -> bool {
        if cdl_no_wick(open, high, low, close) == NO_WICK {
            return true;
        }

        false
    }
}
