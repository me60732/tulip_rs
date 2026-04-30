use crate::cdlcommon::{cdl_wick_length, cdl_body_fill, FILL, HALLOW, SHORT, cdl_height, LONG};
use crate::cldcommontypes::CandleStick;
use crate::candle_types::doji::{CDLDoji, DojiOptions};
pub enum CDLSpinngingTop {
    WhiteSpinningTop,
    BlackSpinningTop,
    HighWave,
}
impl CandleStick for CDLSpinngingTop {
    type Classified = CDLSpinngingTop;
    type Options = DojiOptions;
    fn to_string(&self) -> String {
        match self {
            CDLSpinngingTop::WhiteSpinningTop => "White Spinning Top".to_string(),
            CDLSpinngingTop::BlackSpinningTop => "Black Spinning Top".to_string(),
            CDLSpinngingTop::HighWave => "High Wave".to_string(),
        }
    }
    #[inline(always)]
    fn classify(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> Option<Self::Classified> {
        if CDLSpinngingTop::is_spinning_top(open, high, low, close, options) {
            if CDLSpinngingTop::is_white_spinning_top(open, high, low, close, options) {
                return Some(CDLSpinngingTop::WhiteSpinningTop);
            } else if CDLSpinngingTop::is_black_spinning_top(open, high, low, close, options) {
                return Some(CDLSpinngingTop::BlackSpinningTop);
            } else if CDLSpinngingTop::is_high_wave(open, high, low, close, options) {
                return Some(CDLSpinngingTop::HighWave);
            }
        }
        None
    }
}
impl CDLSpinngingTop {
    #[inline(always)]
    fn is_spinning_top(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if CDLDoji::is_candlestick(open, high, low, close, options) { return false }
        if cdl_wick_length((open, close), low, Some(1.00000001)) != SHORT
        || cdl_wick_length((open, close), high, Some(1.0000001)) != SHORT 
        { return true }
        false
    }
    #[inline(always)]
    fn is_white_spinning_top(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_body_fill(open, close) == FILL { return false }
        
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG 
        && (cdl_wick_length((open, close), low, Some(3.0)) == LONG 
        || cdl_wick_length((open, close), high, Some(3.0)) == LONG) 
        { return false }
        true
    }
    #[inline(always)]
    fn is_black_spinning_top(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_body_fill(open, close) == HALLOW { return false }
        
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG 
        && (cdl_wick_length((open, close), low, Some(3.0)) == LONG 
        || cdl_wick_length((open, close), high, Some(3.0)) == LONG) 
        { return false }
        true
    }
    #[inline(always)]
    fn is_high_wave(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == SHORT { return false }
        
        if cdl_wick_length((open, close), low, Some(3.0)) == SHORT 
        && cdl_wick_length((open, close), high, Some(3.0)) == SHORT 
        { return false }
        true
    }
}