use crate::cldcommontypes::CandleStick;
use crate::cdlcommon::{cdl_body_fill, cdl_body_greater, cdl_height, cdl_no_wick, cdl_wick_length, HALLOW, HAS_WICK, LONG, SHORT};
use crate::candle_types::doji::{CDLDoji, DojiOptions};
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
    type Options = DojiOptions;
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
    #[inline(always)]
    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> bool {
        if !CDLDoji::is_candlestick(open, high, low, close, options) && cdl_no_wick(open, high, low, close) == HAS_WICK && cdl_wick_length((open, close), low, None) == SHORT && cdl_wick_length((open, close), high, None) == SHORT {
            return true;
        }
        false
    }
    #[inline(always)]
    fn classify(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> Option<Self::Classified> {
        if CDLBasic::is_candlestick(open, high, low, close, options) {
            if CDLBasic::is_long_body(open, high, low, close, options) {
                if cdl_body_fill(open, close) == HALLOW {
                    return Some(CDLBasic::LongWhiteCandle);
                } else {
                    return Some(CDLBasic::LongBlackCandle);
                }
            } else if CDLBasic::is_long_line(high, low, options) {
                if cdl_body_fill(open, close) == HALLOW {
                    return Some(CDLBasic::WhiteCandle);
                } else {
                    return Some(CDLBasic::BlackCandle);
                }
            } else if cdl_body_fill(open, close) == HALLOW {
                return Some(CDLBasic::ShortWhiteCandle);
            } else {
                return Some(CDLBasic::ShortBlackCandle);
            }
        }
        None
    }
}
impl CDLBasic {
    #[inline(always)]
    fn is_long_body(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {

        if CDLBasic::is_long_line( high, low,  options) {// long line
            if cdl_body_greater((open, close), options.avg_body, 3.0) { // long body
                return true;
            }
        }
        false
    }
    #[inline(always)]
    fn is_long_line(high: f64, low: f64, options: &DojiOptions) -> bool {
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG {// long line
            return true;
        }
        false
    }
}