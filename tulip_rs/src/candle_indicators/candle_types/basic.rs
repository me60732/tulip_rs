use crate::candle_indicators::candle_types::doji::CDLDoji;
use crate::candle_indicators::common::{
    cdl_body_fill, cdl_body_greater, cdl_height, cdl_no_wick, cdl_wick_length, HALLOW, HAS_WICK,
    LONG, SHORT,
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
            && CDLBasic::is_candlestick_fast(open, high, low, close, false, state)
        {
            return true;
        }
        false
    }
    #[inline(always)]
    fn is_candlestick_fast(open: f64, high: f64, low: f64, close: f64, _: bool, _: &State) -> bool {
        if cdl_no_wick(open, high, low, close) == HAS_WICK
            && cdl_wick_length((open, close), low, None) == SHORT
            && cdl_wick_length((open, close), high, None) == SHORT
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
        CDLBasic::classify_fast(open, high, low, close, cdl_body_fill(open, close), state)
    }
    #[inline(always)]
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        fill: bool,
        state: &State,
    ) -> Option<Self::Classified> {
        if CDLBasic::is_candlestick_fast(open, high, low, close, fill, state) {
            if CDLBasic::is_long_body(open, high, low, close, state) {
                if fill == HALLOW {
                    return Some(CDLBasic::LongWhiteCandle);
                } else {
                    return Some(CDLBasic::LongBlackCandle);
                }
            } else if CDLBasic::is_long_line(high, low, state) {
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
    fn is_long_body(open: f64, high: f64, low: f64, close: f64, state: &State) -> bool {
        if CDLBasic::is_long_line(high, low, state) {
            // long line
            if cdl_body_greater((open, close), state.ema_body, 3.0) {
                // long body
                return true;
            }
        }
        false
    }
    #[inline(always)]
    fn is_long_line(high: f64, low: f64, state: &State) -> bool {
        if cdl_height((high, low), state.ema_line) == LONG {
            // long line
            return true;
        }
        false
    }
}
