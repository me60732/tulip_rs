use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::ema::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::zlema_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    real: Vec<f64>,
    lag: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, lag: usize) -> Self {
        Self {
            state,
            real: real[real.len() - lag..].to_vec(),
            lag,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub zlema: f64,
    pub per: f64,
    pub multiplier: f64,
}
impl State {
    pub fn new(real: &[f64], lag: usize, period: usize) -> Self {
        let (multiplier, per) = multiplier(period);
        Self {
            zlema: real[lag - 1],
            multiplier,
            per,
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, current: f64, lagged: f64) -> f64 {
        let adjusted = current + (current - lagged);

        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        self.zlema
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        // Merge stored trailing real values with new input.
        self.real.extend_from_slice(inputs[0]);

        let mut zlema_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_zlema(&self.real, self.lag, &mut self.state, &mut zlema_line);

        self.real.drain(..self.real.len() - self.lag);

        Ok(vec![zlema_line])
    }
}
pub fn info() -> Info<'static> {
    Info {
        name: "zlema",
        full_name: "Zero Lag Exponential Moving Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        // One input: real (can be any price series).
        inputs: &["real"],
        // One option: period.
        options: &["period"],
        outputs: &["zlema"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum required data points.
/// We require that the input length reaches lag, where lag = max((period - 1)/2, 1).
pub fn min_data(options: &[f64]) -> usize {
    ((options[0] as usize - 1) / 2) + 1
}

/// Returns the output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    let lag = ((period.saturating_sub(1)) / 2).max(1);
    
    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut zlema_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::new(real, lag, period);

    cycle_zlema(real, lag, &mut state, &mut zlema_line);

    Ok((vec![zlema_line], IndicatorState::new(real, state, lag)))
}

/// Iterates over the real array (starting at index 0 of the slice)
/// to compute ZLEMA using the provided initial `prev_zlema`.
/// Calls the refactored `calc` function for each new value.
/// Returns the final ZLEMA value.
fn cycle_zlema(real: &[f64], lag: usize, state: &mut State, zlema_line: &mut [f64]) {
    for (j, i) in (lag..real.len()).enumerate() {
        unsafe {
            *zlema_line.get_unchecked_mut(j) =
                state.calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Calculate a single ZLEMA value using the previous value, the current real value,
/// and the lagged real value.
#[inline(always)]
pub fn calc(state: &mut State, current: f64, lagged: f64) -> f64 {
    state.calc(current, lagged)
}
