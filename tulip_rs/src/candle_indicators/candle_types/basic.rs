use crate::candle_indicators::candle_types::doji::CDLDoji;
use crate::candle_indicators::common::{
    BOTH_WICK, CandleShape, HALLOW, LONG, SHORT, cdl_body_greater
};
use crate::candle_indicators::pattern_test::EmaState as State;

use crate::candle_indicators::types::CandleStick;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CDLBasic {
    ShortWhiteCandle,
    WhiteCandle,
    LongWhiteCandle,
    ShortBlackCandle,
    BlackCandle,
    LongBlackCandle,
}

impl CandleStick for CDLBasic {
    type Classified = CDLBasic;
    //type Options = DojiOptions;
    fn to_string(&self) -> String {
        match self {
            CDLBasic::ShortWhiteCandle => "Short White Candle".to_string(),
            CDLBasic::WhiteCandle => "White Candle".to_string(),
            CDLBasic::LongWhiteCandle => "Long White Candle".to_string(),
            CDLBasic::ShortBlackCandle => "Short Black Candle".to_string(),
            CDLBasic::BlackCandle => "Black Candle".to_string(),
            CDLBasic::LongBlackCandle => "Long Black Candle".to_string(),
        }
    }

    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, state: &State) -> bool {
        if !CDLDoji::is_candlestick(open, high, low, close, state)
            && CDLBasic::is_candlestick_fast(open, high, low, close, &mut CandleShape::default(), state)
        {
            return true;
        }
        false
    }
    #[inline(always)]
    fn is_candlestick_fast(open: f64, high: f64, low: f64, close: f64, candle_shape: &mut CandleShape, _: &State) -> bool {
        
        if candle_shape.get_wick(open, high, low, close) == BOTH_WICK
        && candle_shape.get_bottom_wick_length(open, low, close) == SHORT
        && candle_shape.get_top_wick_length(open, high, close) == SHORT
        {
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
        if CDLDoji::is_candlestick(open, high, low, close, state) {
            return None;
        }
        CDLBasic::classify_fast(open, high, low, close, &mut CandleShape::default(), state)
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
        if CDLBasic::is_candlestick_fast(open, high, low, close, candle_shape, state) {
            let fill = candle_shape.get_fill(open, close);
            if CDLBasic::is_long_body(open, close, state, &candle_shape) {
                if fill == HALLOW {
                    return Some(CDLBasic::LongWhiteCandle);
                } else {
                    return Some(CDLBasic::LongBlackCandle);
                }
            } else if CDLBasic::is_long_line(&candle_shape) {
                if fill == HALLOW {
                    return Some(CDLBasic::WhiteCandle);
                } else {
                    return Some(CDLBasic::BlackCandle);
                }
            } else if fill == HALLOW {
                return Some(CDLBasic::ShortWhiteCandle);
            } else {
                return Some(CDLBasic::ShortBlackCandle);
            }
        }
        None
    }
    #[inline(always)]
    fn discriminant(&self) -> u8 {
        match self {
            CDLBasic::ShortWhiteCandle => 0,
            CDLBasic::WhiteCandle => 1,
            CDLBasic::LongWhiteCandle => 2,
            CDLBasic::ShortBlackCandle => 3,
            CDLBasic::BlackCandle => 4,
            CDLBasic::LongBlackCandle => 5,
        }
    }
    #[inline(always)]
    fn to_bit(&self) -> u8 {
        match self {
            CDLBasic::ShortWhiteCandle => 1 << 0, // Bit 0
            CDLBasic::WhiteCandle => 1 << 1,      // Bit 1
            CDLBasic::LongWhiteCandle => 1 << 2,  // Bit 2
            CDLBasic::ShortBlackCandle => 1 << 3, // Bit 3
            CDLBasic::BlackCandle => 1 << 4,      // Bit 4
            CDLBasic::LongBlackCandle => 1 << 5,  // Bit 5
        }
    }
    #[inline(always)]
    fn from_bit(bits: u8) -> Option<Self::Classified> {
        match bits {
            b if b & (1 << 0) != 0 => Some(CDLBasic::ShortWhiteCandle),
            b if b & (1 << 1) != 0 => Some(CDLBasic::WhiteCandle),
            b if b & (1 << 2) != 0 => Some(CDLBasic::LongWhiteCandle),
            b if b & (1 << 3) != 0 => Some(CDLBasic::ShortBlackCandle),
            b if b & (1 << 4) != 0 => Some(CDLBasic::BlackCandle),
            b if b & (1 << 5) != 0 => Some(CDLBasic::LongBlackCandle),
            _ => None,
        }
    }
}
impl CDLBasic {
    #[inline(always)]
    fn is_long_body(open: f64, close: f64, state: &State, candle_shape: &CandleShape) -> bool {
        if CDLBasic::is_long_line(candle_shape) {
            // long line
            if cdl_body_greater((open, close), state.ema_body, 3.0) {
                // long body
                return true;
            }
        }
        false
    }
    #[inline(always)]
    fn is_long_line(candle_shape: &CandleShape) -> bool {
        if candle_shape.line_height == LONG {
            // long line
            return true;
        }
        false
    }
}
