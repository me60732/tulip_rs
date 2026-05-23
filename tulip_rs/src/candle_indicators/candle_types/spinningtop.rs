use crate::candle_indicators::candle_types::doji::CDLDoji;
use crate::candle_indicators::common::{
    cdl_height, cdl_wick_length, CandleShape, FILL, HALLOW, LONG, SHORT,
};
use crate::candle_indicators::{pattern_test::EmaState as State, types::CandleStick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CDLSpinningTop {
    WhiteSpinningTop,
    BlackSpinningTop,
    HighWave,
}
impl CandleStick for CDLSpinningTop {
    type Classified = CDLSpinningTop;
    fn to_string(&self) -> String {
        match self {
            CDLSpinningTop::WhiteSpinningTop => "White Spinning Top".to_string(),
            CDLSpinningTop::BlackSpinningTop => "Black Spinning Top".to_string(),
            CDLSpinningTop::HighWave => "High Wave".to_string(),
        }
    }
    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, state: &State) -> bool {
        if CDLDoji::is_candlestick(open, high, low, close, state) {
            return false;
        }
        CDLSpinningTop::is_candlestick_fast(
            open,
            high,
            low,
            close,
            &mut CandleShape::default(),
            state,
        )
    }
    #[inline(always)]
    fn is_candlestick_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        _: &State,
    ) -> bool {
        if candle_shape.get_bottom_wick_length(open, low, close) != SHORT
            || candle_shape.get_top_wick_length(open, high, close) != SHORT
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
        CDLSpinningTop::classify_fast(open, high, low, close, &mut CandleShape::default(), state)
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
        if CDLSpinningTop::is_candlestick_fast(open, high, low, close, candle_shape, state) {
            if CDLSpinningTop::is_white_spinning_top(open, high, low, close, candle_shape) {
                return Some(CDLSpinningTop::WhiteSpinningTop);
            } else if CDLSpinningTop::is_black_spinning_top(
                open,
                high,
                low,
                close,
                candle_shape,
            ) {
                return Some(CDLSpinningTop::BlackSpinningTop);
            } else if CDLSpinningTop::is_high_wave(open, high, low, close, state, &candle_shape) {
                return Some(CDLSpinningTop::HighWave);
            }
        }
        None
    }
    #[inline(always)]
    fn discriminant(&self) -> u8 {
        match self {
            CDLSpinningTop::WhiteSpinningTop => 0,
            CDLSpinningTop::BlackSpinningTop => 1,
            CDLSpinningTop::HighWave => 2,
        }
    }
    #[inline(always)]
    fn to_bit(&self) -> u8 {
        match self {
            CDLSpinningTop::WhiteSpinningTop => 1 << 0, // Bit 0
            CDLSpinningTop::BlackSpinningTop => 1 << 1, // Bit 1
            CDLSpinningTop::HighWave => 1 << 2,         // Bit 2
        }
    }
    #[inline(always)]
    fn from_bit(bits: u8) -> Option<Self::Classified> {
        match bits {
            b if b & (1 << 0) != 0 => Some(CDLSpinningTop::WhiteSpinningTop),
            b if b & (1 << 1) != 0 => Some(CDLSpinningTop::BlackSpinningTop),
            b if b & (1 << 2) != 0 => Some(CDLSpinningTop::HighWave),
            _ => None,
        }
    }
}
impl CDLSpinningTop {
    #[inline(always)]
    fn is_white_spinning_top(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
    ) -> bool {
        if candle_shape.get_fill(open, close) == FILL {
            return false;
        }
        if candle_shape.line_height == LONG
            && (cdl_wick_length((open, close), low, Some(3.0)) == LONG
                || cdl_wick_length((open, close), high, Some(3.0)) == LONG)
        {
            return false;
        }
        true
    }
    #[inline(always)]
    fn is_black_spinning_top(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
    ) -> bool {
        if candle_shape.get_fill(open, close) == HALLOW {
            return false;
        }

        if candle_shape.line_height == LONG
            && (cdl_wick_length((open, close), low, Some(3.0)) == LONG
                || cdl_wick_length((open, close), high, Some(3.0)) == LONG)
        {
            return false;
        }
        true
    }
    #[inline(always)]
    fn is_high_wave(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        state: &State,
        candle_shape: &CandleShape,
    ) -> bool {
        if candle_shape.line_height == SHORT {
            return false;
        }
        if cdl_height((open, close), state.ema_body) == LONG {
            return false;
        }

        if cdl_wick_length((open, close), low, Some(3.0)) == SHORT
            && cdl_wick_length((open, close), high, Some(3.0)) == SHORT
        {
            return false;
        }
        true
    }
}
