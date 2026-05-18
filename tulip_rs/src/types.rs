use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy)]
pub enum DisplayType {
    Overlay,
    Indicator,
    Math,
}
impl fmt::Display for DisplayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndicatorType {
    Trend,
    Momentum,
    Volume,
    Volatility,
    Price,
    Cycle,
    CandleStick,
    Other,
}
impl fmt::Display for IndicatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug)]
pub struct Output {
    pub indicators: Vec<Vec<f64>>,
    pub state: IndicatorState,
}

pub struct Info<'a> {
    pub name: &'a str,
    pub full_name: &'a str,
    pub display_type: DisplayType,
    pub indicator_type: IndicatorType,
    pub inputs: &'a [&'a str],
    pub options: &'a [&'a str],
    pub outputs: &'a [&'a str],
    pub optional_outputs: &'a [&'a str],
}
pub struct InfoIndicatorState<'a> {
    pub array_values: Option<&'a [&'a str]>,
    pub single_values: Option<&'a [&'a str]>,
}
impl<'a> InfoIndicatorState<'a> {
    /// Returns the array values or an empty array if None.
    pub fn get_array_values(&self) -> &[&'a str] {
        self.array_values.unwrap_or(&[])
    }

    /// Returns the single values or an empty array if None.
    pub fn get_single_values(&self) -> &[&'a str] {
        self.single_values.unwrap_or(&[])
    }
}
#[derive(Debug, Clone)]
pub struct IndicatorState {
    pub single_values: Option<Vec<f64>>,
    pub array_values: Option<Vec<Vec<f64>>>,
}

pub trait IndicatorFromState {
    fn indicator_from_state(
        &mut self,
        inputs: Vec<&Vec<f64>>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError>;
}

impl IndicatorState {
    pub fn new(single_values: Option<Vec<f64>>, array_values: Option<Vec<Vec<f64>>>) -> Self {
        Self {
            single_values,
            array_values,
        }
    }
    pub fn single_values(&self) -> &[f64] {
        self.single_values.as_deref().unwrap_or(&[])
    }
    pub fn array_values(&self) -> &[Vec<f64>] {
        self.array_values.as_deref().unwrap_or(&[])
    }
}

impl IndicatorError {
    pub fn message(&self) -> &str {
        match self {
            IndicatorError::InvalidInputs => {
                "Invalid inputs provided for the indicator calculation"
            }
            IndicatorError::NotEnoughData => {
                "Not enough data input parameter"
            }
            IndicatorError::InvalidOptions => "Invalid options provided",
            IndicatorError::InvalidIndicatorState => "Invalid state inputs provided",
        }
    }
}

impl fmt::Display for IndicatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

// Custom error type for indicator calculations.
#[derive(Debug)]
pub enum IndicatorError {
    InvalidInputs,
    NotEnoughData,
    InvalidOptions,
    InvalidIndicatorState,
}

impl Error for IndicatorError {}

pub enum IndicatorInfoOrInteger<'a> {
    Info(&'a Info<'a>),
    Integer(usize),
}

pub trait IndicatorStateDeref {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
