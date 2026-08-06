use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, TState, Indicator, IndicatorResult};
pub use crate::indicators::sma::State as SmaState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::qstick_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    open: Vec<f64>,
    close: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
#[repr(transparent)]
pub struct State<S = Cold>(pub SmaState<S>);
impl<S> Deref for State<S> {
    type Target = SmaState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl State<Cold> {
    pub fn init_state(open: &[f64], close: &[f64], period: usize) -> State<Warm> {
        let mut sum = 0.0;
        for i in 0..period {
            sum += close[i] - open[i];
        }
        State(SmaState::new(sum, period).into_warm())
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (open, close, prev_open, prev_close): Self::Inputs<'a>
    ) -> Self::Outputs {
        self.0.calc((close - open, prev_close - prev_open))
    }
}


impl IndicatorState {
    pub fn new(open: &[f64], close: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            open: open[open.len() - period..].to_vec(),
            close: close[close.len() - period..].to_vec(),
            period,
            state
        }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.open.extend_from_slice(inputs[0]);
        self.close.extend_from_slice(inputs[1]);

        let mut qstick_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };
        
        cycle_qstick(
            &self.open,
            &self.close,
            self.period,
            &mut self.state,
            &mut qstick_line,
        );

        self.close.drain(..self.close.len() - self.period);
        self.open.drain(..self.open.len() - self.period);

        Ok(vec![qstick_line])
    }
}



/// Performs the main calculation loop for the QStick indicator.
///
/// # Arguments
///
/// * `open` - A slice containing the open prices.
/// * `close` - A slice containing the close prices.
/// * `period` - The period for the QStick calculation.
/// * `multiplier` - The multiplier for averaging (1/period).
/// * `qstick_line` - A mutable slice to store the QStick values.
/// * `sum` - The running sum of close-open differences.
///
/// # Returns
///
/// The updated running sum.
fn cycle_qstick(
    open: &[f64],
    close: &[f64],
    period: usize,
    state: &mut State<Warm>,
    qstick_line: &mut [f64],
) {
    for (j, i) in (period..open.len()).enumerate() {
        unsafe {
            *qstick_line.get_unchecked_mut(j) = state.calc((
                *open.get_unchecked(i),
                *close.get_unchecked(i),
                *open.get_unchecked(j),
                *close.get_unchecked(j),
            ));
        }
    }
}


pub struct QStick;
impl Indicator<INPUTS, OPTIONS> for QStick {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "qstick",
        full_name: "QStick",
        indicator_type: IndicatorType::Momentum,
        inputs: &["open", "close"],
        options: &["period"],
        outputs: &["qstick"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "qstick",
            label: "QSTICK",
            display_type: DisplayType::Indicator,
            outputs: &["qstick"],
        }],
    };
    
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let [open, close] = *inputs;

        let mut qstick_line = {
            let capacity = Self::output_length(open.len(), options);
            crate::uninit_vec!(f64, capacity)
        };
    
        let mut state = State::init_state(open, close, period);
        cycle_qstick(open, close, period, &mut state, &mut qstick_line);
    
        Ok((
            vec![qstick_line],
            IndicatorState::new(open, close, state, period),
        ))
    }
}