use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy)]
pub enum DisplayType {
    Overlay,
    Indicator,
    Volume
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
    Math,
    Other,
}
impl fmt::Display for IndicatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy)]
pub struct Info {
    pub name: &'static str,
    pub full_name: &'static str,
    pub indicator_type: IndicatorType,
    pub inputs: &'static [&'static str],
    pub options: &'static [&'static str],
    pub outputs: &'static [&'static str],
    pub optional_outputs: &'static [&'static str],
    pub display_groups: &'static [DisplayGroup],
}
/// Groups one or more related outputs that should be rendered together on the
/// same pane.  A [`DisplayGroup`] can contain both mandatory outputs (from
/// [`Info::outputs`]) and optional outputs (from [`Info::outputs_optional`]),
/// so a consumer must be prepared to render fewer series than listed if some
/// optional outputs were not requested.
///
/// Consumers should use [`DisplayGroup::display_type`] to decide whether to
/// place the group on the main price pane (`Overlay`) or a separate sub-pane
/// (`Indicator`), and [`DisplayGroup::label`] as the human-readable pane title.
///
/// # Fields
/// * `id` — stable, machine-readable key used to identify the group in client
///   code (e.g. `"adx_dx"`, `"true_range"`).
/// * `label` — human-readable title suitable for display in a UI
///   (e.g. `"Directional Index"`, `"True Range"`).
/// * `display_type` — whether the group belongs on the price overlay or a
///   dedicated indicator sub-pane.
/// * `outputs` — the output names belonging to this group.  May include a mix
///   of mandatory and optional outputs; render only the series that were
///   actually computed.
#[derive(Clone, Copy)]
pub struct DisplayGroup {
    pub id: &'static str,                 // machine-readable key, e.g. "emas", "ad"
    pub label: &'static str,              // human-readable pane title, e.g. "AD EMAs"
    pub display_type: DisplayType,        // Overlay | Indicator for this pane
    pub outputs: &'static [&'static str], // subset of optional_outputs that belong here
}


impl IndicatorError {
    pub fn message(&self) -> &str {
        match self {
            IndicatorError::InvalidInputs => {
                "Invalid inputs provided for the indicator calculation"
            }
            IndicatorError::NotEnoughData => "Not enough data input parameter",
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

pub enum IndicatorInfoOrInteger {
    Info(Info),
    Integer(usize),
}

pub trait IndicatorStateDeref {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
