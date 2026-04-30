use crate::types::Info;
use std::fmt;
use std::ops::{Deref, DerefMut};
use crate::indicator_types::TIndicatorState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    state: State,
    doji_max_height: f64,
    min_cdl_height: f64,
    min_cdl_height_tolerance: f64,
    line_multiplier: f64,
    body_multiplier: f64
}
impl IndicatorState {
    pub fn new (inputs: (&[f64], &[f64], &[f64], &[f64]), bars: usize, state: State, doji_max_height: f64, min_cdl_height: f64, min_cdl_height_tolerance: f64) -> Self {
        let (open, high, low, close) = inputs;
        Self {
            open: open[open.len()-bars..].to_vec(),
            high: high[high.len()-bars..].to_vec(),
            low: low[low.len()-bars..].to_vec(),
            close: close[close.len()-bars..].to_vec(),
            state,
            doji_max_height,
            min_cdl_height,
            min_cdl_height_tolerance
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub avg_line: f64,
    pub avg_body: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum ForcastType {
    BearishReversal,
    BullishReversal,
    BearishContinuation,
    BullishContinuation,
    BearishReversalOrContinuation,
    BullishReversalOrContinuation,
}
impl fmt::Display for ForcastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum TrendType {
    Uptrend,
    Downtrend,
    Trend,
}
impl fmt::Display for TrendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
pub struct CandleInfo {
    pub parent: Info<'static>,
    pub forcast: ForcastType,
    pub prior_trend: TrendType,
    pub bars: usize,
    pub japanese_name: &'static str,
    pub crossover_offset: Option<usize>,
}
impl Deref for CandleInfo {
    type Target = Info<'static>;
    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}


pub trait CandleStick{
    type Classified;
    type Options;
    fn classify(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> Option<Self::Classified>;
    #[inline(always)]
    fn is_candlestick(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> bool {
        Self::classify(open, high, low, close, options).is_some()
    }
    fn to_string(&self) -> String;
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
    pub fn new(avg_line: f64, min_line_height: f64, min_line_height_tolerance: f64, avg_body: f64) -> Self {
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
