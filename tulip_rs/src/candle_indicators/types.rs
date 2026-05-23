use crate::candle_indicators::{
    candle_patterns::CandlePattern,
    candle_types::{CDLBasic, CDLDoji, CDLMarubozu, CDLSpinningTop},
    common::CandleShape,
    pattern_test::EmaState as State,
};
//use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Candle type classification - a bar can only be ONE type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleTypes {
    Basic(CDLBasic),
    Doji(CDLDoji),
    Marubozu(CDLMarubozu),
    SpinningTop(CDLSpinningTop),
    Other,
}
impl Default for CandleTypes {
    fn default() -> Self {
        CandleTypes::Other
    }
}
impl CandleTypes {
    /*pub fn get_type(open: f64, high: f64, low: f64, close: f64, state: &State) -> Self {
        let mut candle_shape = CandleShape::new();
        // Check in priority order: Doji -> Marubozu -> SpinningTop -> Basic -> Other
        if let Some(doji) = CDLDoji::classify_fast(open, high, low, close, &mut candle_shape, state) {
            return Self::Doji(doji);
        }
        
        
        if let Some(basic) = CDLBasic::classify_fast(open, high, low, close, &mut candle_shape, state) {
            return Self::Basic(basic);
        }

        if let Some(marubozu) = CDLMarubozu::classify_fast(open, high, low, close, &mut candle_shape, state) {
            return Self::Marubozu(marubozu);
        }

        if let Some(spinning_top) =
            CDLSpinningTop::classify_fast(open, high, low, close, &mut candle_shape, state)
        {
            return Self::SpinningTop(spinning_top);
        }

        Self::Other
    }*/
    pub fn get_type_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        state: &State,
    ) -> Self {
        // Check in priority order: Doji -> Marubozu -> SpinningTop -> Basic -> Other
        if let Some(doji) = CDLDoji::classify_fast(open, high, low, close, candle_shape, state) {
            return Self::Doji(doji);
        }

        if let Some(basic) = CDLBasic::classify_fast(open, high, low, close, candle_shape, state) {
            return Self::Basic(basic);
        }

        if let Some(marubozu) = CDLMarubozu::classify_fast(open, high, low, close, candle_shape, state) {
            return Self::Marubozu(marubozu);
        }

        if let Some(spinning_top) =
            CDLSpinningTop::classify_fast(open, high, low, close, candle_shape, state)
        {
            return Self::SpinningTop(spinning_top);
        }

        Self::Other
    }
}
/// Pattern for matching candle types with bitmask support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleTypePattern {
    Basic(u8),       // Bitmask of CDLBasic variants
    Doji(u8),        // Bitmask of CDLDoji variants
    Marubozu(u8),    // Bitmask of CDLMarubozu variants
    SpinningTop(u8), // Bitmask of CDLSpinningTop variants
    Any,             // Accept any candle type
}

impl CandleTypePattern {
    /// Check if this pattern matches the actual candle type
    pub fn matches(&self, actual: &CandleTypes) -> bool {
        match (self, actual) {
            (CandleTypePattern::Any, _) => true,

            (CandleTypePattern::Marubozu(mask), CandleTypes::Marubozu(variant)) => {
                (mask & variant.to_bit()) != 0
            }

            (CandleTypePattern::Doji(mask), CandleTypes::Doji(variant)) => {
                (mask & variant.to_bit()) != 0
            }

            (CandleTypePattern::SpinningTop(mask), CandleTypes::SpinningTop(variant)) => {
                (mask & variant.to_bit()) != 0
            }

            (CandleTypePattern::Basic(mask), CandleTypes::Basic(variant)) => {
                (mask & variant.to_bit()) != 0
            }

            // Type mismatch - pattern wants Marubozu but bar is Doji, etc.
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastType {
    BearishReversal,
    BullishReversal,
    BearishContinuation,
    BullishContinuation,
    BearishReversalOrContinuation,
    BullishReversalOrContinuation,
}
impl fmt::Display for ForecastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ForecastType {
    /// Get all forecast types that are valid for a given trend when no specific forecast is requested.
    /// This is used when filtering by trend alone (no forecast type specified).
    #[inline(always)]
    pub const fn all_for_trend(is_uptrend: bool) -> &'static [ForecastType] {
        if is_uptrend {
            &[
                ForecastType::BearishReversal,     // reverses FROM uptrend TO downtrend
                ForecastType::BullishContinuation, // continues uptrend
                ForecastType::BearishReversalOrContinuation,
                ForecastType::BullishReversalOrContinuation,
            ]
        } else {
            &[
                ForecastType::BullishReversal,     // reverses FROM downtrend TO uptrend
                ForecastType::BearishContinuation, // continues downtrend
                ForecastType::BearishReversalOrContinuation,
                ForecastType::BullishReversalOrContinuation,
            ]
        }
    }

    /// Check if this forecast type is valid for the given prior trend.
    /// Returns true if the pattern with this forecast can occur in the given trend context.
    #[inline(always)]
    pub const fn matches_trend(&self, is_uptrend: bool) -> bool {
        match self {
            ForecastType::BearishReversal => is_uptrend, // requires uptrend to reverse from
            ForecastType::BullishReversal => !is_uptrend, // requires downtrend to reverse from
            ForecastType::BearishContinuation => !is_uptrend, // requires downtrend to continue
            ForecastType::BullishContinuation => is_uptrend, // requires uptrend to continue
            ForecastType::BearishReversalOrContinuation => true, // valid in any trend
            ForecastType::BullishReversalOrContinuation => true, // valid in any trend
        }
    }
}

pub struct CandleInfo {
    pub name: &'static str,
    pub full_name: &'static str,
    pub forecast: ForecastType,
    pub bars: usize,
    pub extended_pattern: Option<CandlePattern>,
    pub japanese_name: &'static str,
}

pub trait CandleStick {
    type Classified;
    fn classify(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        state: &State,
    ) -> Option<Self::Classified>;
    /// Does not preform Doji test, Doji candle must have already been eliminated, and fill already determined
    fn classify_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        state: &State,
    ) -> Option<Self::Classified>;
    /// Does not preform Doji test, Doji candle must have already been eliminated
    #[inline(always)]
    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, state: &State) -> bool {
        Self::classify(open, high, low, close, state).is_some()
    }
    #[inline(always)]
    fn is_candlestick_fast(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_shape: &mut CandleShape,
        state: &State,
    ) -> bool {
        Self::classify_fast(open, high, low, close, candle_shape, state).is_some()
    }
    fn to_string(&self) -> String;
    fn discriminant(&self) -> u8;
    fn to_bit(&self) -> u8;
    fn from_bit(bits: u8) -> Option<Self::Classified>;
}

pub struct LineOptions {
    pub avg_line: f64,
    pub min_cdl_height: f64,
    pub min_cdl_height_tolerance: f64,
}
impl LineOptions {
    pub fn new(avg_line: f64, min_line_height: f64, min_line_height_tolerance: f64) -> Self {
        Self {
            avg_line,
            min_cdl_height: min_line_height,
            min_cdl_height_tolerance: min_line_height_tolerance,
        }
    }
}
pub struct BodyOptions {
    pub line_options: LineOptions,
    pub avg_body: f64,
    //pub min_body_height: f64,
}
impl BodyOptions {
    pub fn new(
        avg_line: f64,
        min_line_height: f64,
        min_line_height_tolerance: f64,
        avg_body: f64,
    ) -> Self {
        Self {
            line_options: LineOptions::new(avg_line, min_line_height, min_line_height_tolerance),
            avg_body,
        }
    }
}
impl Deref for BodyOptions {
    type Target = LineOptions;
    fn deref(&self) -> &Self::Target {
        &self.line_options
    }
}

impl DerefMut for BodyOptions {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.line_options
    }
}
