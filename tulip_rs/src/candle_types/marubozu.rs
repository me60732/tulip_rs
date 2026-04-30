use crate::cldcommontypes::CandleStick;
use crate::cdlcommon::{cdl_body_fill, cdl_no_wick, cdl_wick_length, HALLOW, NO_TOP_WICK, NO_WICK, SHORT, NO_BOTTOM_WICK};
use crate::candle_types::doji::{CDLDoji, DojiOptions};
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
    type Options = DojiOptions;
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
    #[inline(always)]
    fn classify(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> Option<Self::Classified> {
        if !CDLDoji::is_candlestick(open, high, low, close, options) {
            if cdl_no_wick(open, high, low, close) == NO_WICK {
                if cdl_body_fill(open, close) == HALLOW {
                    return Some(CDLMarubozu::WhiteMarubozu);
                } else {
                    return Some(CDLMarubozu::BlackMarubozu);
                }
            } else if cdl_no_wick(open, high, low, close) == NO_TOP_WICK && cdl_wick_length((open, close), low, None) == SHORT {
                if cdl_body_fill(open, close) == HALLOW {
                    return Some(CDLMarubozu::ClosingWhiteMarubozu);
                } else {
                    return Some(CDLMarubozu::OpeningBlackMarubozu);
                }
            } else if cdl_no_wick(open, high, low, close) == NO_BOTTOM_WICK && cdl_wick_length((open, close), high, None) == SHORT {
                if cdl_body_fill(open, close) == HALLOW {
                    return Some(CDLMarubozu::OpeningWhiteMarubozu);
                } else {
                    return Some(CDLMarubozu::ClosingBlackMarubozu);
                }

            }
        }
        None
    }
}