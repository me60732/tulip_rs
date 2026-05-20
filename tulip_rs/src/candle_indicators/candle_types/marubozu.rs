use crate::candle_indicators::candle_types::doji::CDLDoji;
use crate::candle_indicators::common::{
    CandleShape, HALLOW, NO_BOTTOM_WICK, NO_TOP_WICK,
    NO_WICK, SHORT,
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
        CDLMarubozu::classify_fast(open, high, low, close, &mut CandleShape::default(), state)
    }
    #[inline(always)]
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        _: &State,
    ) -> Option<Self::Classified> {
        let wick = candle_shape.get_wick(open, high, low, close);
        if wick == NO_WICK {
            if candle_shape.get_fill(open, close) == HALLOW {
                return Some(CDLMarubozu::WhiteMarubozu);
            } else {
                return Some(CDLMarubozu::BlackMarubozu);
            }
        } else if wick == NO_TOP_WICK
            && candle_shape.get_bottom_wick_length(open, low, close) == SHORT
        {
            if candle_shape.get_fill(open, close) == HALLOW {
                return Some(CDLMarubozu::ClosingWhiteMarubozu);
            } else {
                return Some(CDLMarubozu::OpeningBlackMarubozu);
            }
        } else if wick == NO_BOTTOM_WICK
            && candle_shape.get_top_wick_length(open, high, close) == SHORT
        {
            if candle_shape.get_fill(open, close) == HALLOW {
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
