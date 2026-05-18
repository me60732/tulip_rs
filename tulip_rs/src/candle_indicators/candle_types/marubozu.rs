use crate::candle_indicators::candle_types::doji::CDLDoji;
use crate::candle_indicators::common::{
    cdl_body_fill, cdl_no_wick, cdl_wick_length, HALLOW, NO_BOTTOM_WICK, NO_TOP_WICK, NO_WICK,
    SHORT,
};
use crate::candle_indicators::{pattern_test::EmaState as State, types::CandleStick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CDLMarubozu {
    WhiteMarubozu,
    OpeningWhiteMarubozu,
    ClosingWhiteMarubozu,
    BlackMarubozu,
    OpeningBlackMarubozu,
    ClosingBlackMarubozu,
}

impl CandleStick for CDLMarubozu {
    type Classified = CDLMarubozu;
    fn to_string(&self) -> String {
        match self {
            CDLMarubozu::WhiteMarubozu => "White Marubozu".to_string(),
            CDLMarubozu::OpeningWhiteMarubozu => "Opening White Marubozu".to_string(),
            CDLMarubozu::ClosingWhiteMarubozu => "Closing White Marubozu".to_string(),
            CDLMarubozu::BlackMarubozu => "Black Marubozu".to_string(),
            CDLMarubozu::OpeningBlackMarubozu => "Opening Black Marubozu".to_string(),
            CDLMarubozu::ClosingBlackMarubozu => "Closing Black Marubozu".to_string(),
        }
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
        CDLMarubozu::classify_fast(open, high, low, close, cdl_body_fill(open, close), state)
    }
    #[inline(always)]
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        fill: bool,
        _: &State,
    ) -> Option<Self::Classified> {
        if cdl_no_wick(open, high, low, close) == NO_WICK {
            if fill == HALLOW {
                return Some(CDLMarubozu::WhiteMarubozu);
            } else {
                return Some(CDLMarubozu::BlackMarubozu);
            }
        } else if cdl_no_wick(open, high, low, close) == NO_TOP_WICK
            && cdl_wick_length((open, close), low, None) == SHORT
        {
            if fill == HALLOW {
                return Some(CDLMarubozu::ClosingWhiteMarubozu);
            } else {
                return Some(CDLMarubozu::OpeningBlackMarubozu);
            }
        } else if cdl_no_wick(open, high, low, close) == NO_BOTTOM_WICK
            && cdl_wick_length((open, close), high, None) == SHORT
        {
            if fill == HALLOW {
                return Some(CDLMarubozu::OpeningWhiteMarubozu);
            } else {
                return Some(CDLMarubozu::ClosingBlackMarubozu);
            }
        }
        None
    }
    #[inline(always)]
    fn discriminant(&self) -> u8 {
        match self {
            CDLMarubozu::WhiteMarubozu => 0,
            CDLMarubozu::OpeningWhiteMarubozu => 1,
            CDLMarubozu::ClosingWhiteMarubozu => 2,
            CDLMarubozu::BlackMarubozu => 3,
            CDLMarubozu::OpeningBlackMarubozu => 4,
            CDLMarubozu::ClosingBlackMarubozu => 5,
        }
    }
    #[inline(always)]
    fn to_bit(&self) -> u8 {
        match self {
            CDLMarubozu::WhiteMarubozu => 1 << 0,        // Bit 0
            CDLMarubozu::OpeningWhiteMarubozu => 1 << 1, // Bit 1
            CDLMarubozu::ClosingWhiteMarubozu => 1 << 2, // Bit 2
            CDLMarubozu::BlackMarubozu => 1 << 3,        // Bit 3
            CDLMarubozu::OpeningBlackMarubozu => 1 << 4, // Bit 4
            CDLMarubozu::ClosingBlackMarubozu => 1 << 5, // Bit 5
        }
    }
    #[inline(always)]
    fn from_bit(bits: u8) -> Option<Self::Classified> {
        match bits {
            b if b & (1 << 0) != 0 => Some(CDLMarubozu::WhiteMarubozu),
            b if b & (1 << 1) != 0 => Some(CDLMarubozu::OpeningWhiteMarubozu),
            b if b & (1 << 2) != 0 => Some(CDLMarubozu::ClosingWhiteMarubozu),
            b if b & (1 << 3) != 0 => Some(CDLMarubozu::BlackMarubozu),
            b if b & (1 << 4) != 0 => Some(CDLMarubozu::OpeningBlackMarubozu),
            b if b & (1 << 5) != 0 => Some(CDLMarubozu::ClosingBlackMarubozu),
            _ => None,
        }
    }
}
