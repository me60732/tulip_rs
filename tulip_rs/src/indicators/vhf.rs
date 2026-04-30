use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::{
    calc as calc_max, calc_unchecked as calc_max_unchecked, State as MaxState,
};
use crate::indicators::min::{
    calc as calc_min, calc_unchecked as calc_min_unchecked, State as MinState,
};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    real: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, real: &[f64], period: usize) -> Self {
        Self {
            state,
            period,
            real: real[real.len() - period - 1..].to_vec(),
        }
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut vhf_line = crate::uninit_vec!(f64, inputs[0].len());

        match self.period {
            1..=4 => {
                cycle::<1>(&self.real, self.period, &mut self.state, &mut vhf_line);
            }
            5..30 => {
                cycle::<4>(&self.real, self.period, &mut self.state, &mut vhf_line);
            }
            _ => {
                cycle::<8>(&self.real, self.period, &mut self.state, &mut vhf_line);
            }
        }

        self.real.drain(..self.real.len() - self.period - 1);
        Ok(vec![vhf_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
    pub sum: f64,
}

impl State {
    pub fn new(min: (f64, usize), max: (f64, usize), sum: f64) -> Self {
        State {
            min_state: MinState::new(min.0, min.1),
            max_state: MaxState::new(max.0, max.1),
            sum,
        }
    }
}
/// Returns meta-information for this indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "vhf",
        full_name: "Vertical Horizontal Filter",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["real"],
        options: &["period"],
        outputs: &["vhf"],
        optional_outputs: &[],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required by the indicator.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Determines the length of the output given the data and recent-only parameter.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the full dataset outputs for this indicator.
///
/// Performs common validation, determines the start index, prepares output vectors,
/// and does a single-pass loop to calculate the indicator values.
/// Returns an Output struct containing the main indicator outputs and any optional outputs.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    // Validate options and minimal input data.
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    // Determine the start index for processing.
    let real = inputs[0];
    // Prepare the main output vector.
    let mut vhf_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = init_state(real, period, &mut vhf_line);

    match period {
        1..14 => {
            cycle::<1>(real, period, &mut state, &mut vhf_line[1..]);
        }
        14..25 => {
            cycle::<4>(real, period, &mut state, &mut vhf_line[1..]);
        }
        _ => {
            cycle::<8>(real, period, &mut state, &mut vhf_line[1..]);
        }
    }

    Ok((
        vec![vhf_line],
        IndicatorState::new(state, real, period)
    ))
}

pub fn init_state(real: &[f64], period: usize, indicator_line: &mut [f64]) -> State {
    let mut state = State::new((real[0], period), (real[0], period), 0.0);

    for i in 1..=period {
        state.sum += (real[i] - real[i - 1]).abs();
    }
    let (min, _) = calc_min(&mut state.min_state, real, period, (period, period - 1));
    let (max, _) = calc_max(&mut state.max_state, real, period, (period, period - 1));
    let vhf = (max - min) / state.sum.max(f64::EPSILON);
    indicator_line[0] = vhf;
    state
}
/// A common cycle loop through the data.
fn cycle<const N: usize>(real: &[f64], period: usize, state: &mut State, indicator_line: &mut [f64]) {
    let periods = (period, period - 1);
    
    for (j, i) in (period + 1..real.len()).enumerate() {
        unsafe {
            *indicator_line.get_unchecked_mut(j) = calc_unchecked::<N>(
                state,
                (
                    real.get_unchecked(i),
                    real.get_unchecked(i - 1),
                    real.get_unchecked(j+1),//i - period),
                    real.get_unchecked(j)//i - period - 1),
                ),
                real,
                periods,
                i,
            );
        }
    }
}
/// A simple, per-bar calculation function.
#[inline(always)]
pub fn calc(
    state: &mut State,
    values: (&f64, &f64, &f64, &f64),
    real: &[f64],
    periods: (usize, usize),
    i: usize,
) -> f64 {
    let (value, prev_real, old_real, drop_real) = values;
    state.sum += (value - prev_real).abs() - (old_real - drop_real).abs();

    let (min, _) = calc_min(&mut state.min_state, real, i, periods);
    let (max, _) = calc_max(&mut state.max_state, real, i, periods);
    

    (max - min) / state.sum.max(f64::EPSILON)
}
#[inline(always)]
pub unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    values: (&f64, &f64, &f64, &f64),
    real: &[f64],
    periods: (usize, usize),
    i: usize,
) -> f64 {
    let (value, prev_real, old_real, drop_real) = values;
    state.sum += (value - prev_real).abs() - (old_real - drop_real).abs();

    let (min, _) = calc_min_unchecked::<N>(&mut state.min_state, real, i, periods);
    let (max, _) = calc_max_unchecked::<N>(&mut state.max_state, real, i, periods);
    (max - min) / state.sum.max(f64::EPSILON)
}
