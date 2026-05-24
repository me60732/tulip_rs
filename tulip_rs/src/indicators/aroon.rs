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

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
    period: usize,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(high: &[f64], low: &[f64], state: State, period: usize, multiplier: f64) -> Self {
        Self {
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            state,
            period,
            multiplier,
        }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let period = self.period;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let (mut aroon_up_line, mut aroon_down_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        match period {
            1..=4 => {
                cycle_aroon::<1>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    (&mut aroon_down_line, &mut aroon_up_line),
                    &mut self.state,
                );
            }
            5..30 => {
                cycle_aroon::<4>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    (&mut aroon_down_line, &mut aroon_up_line),
                    &mut self.state,
                );
            }
            _ => {
                cycle_aroon::<8>(
                    (&self.high, &self.low),
                    period,
                    self.multiplier,
                    (&mut aroon_down_line, &mut aroon_up_line),
                    &mut self.state,
                );
            }
        }

        self.high.drain(..self.high.len() - period);
        self.low.drain(..self.low.len() - period);

        Ok(vec![aroon_down_line, aroon_up_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
}
impl State {
    pub fn new(min: f64, min_trail: usize, max: f64, max_trail: usize) -> Self {
        State {
            min_state: MinState::new(min, min_trail),
            max_state: MaxState::new(max, max_trail),
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> Self {
        let mut state = Self::new(low[0], period - 1, high[0], period - 1);
        _ = calc_min(&mut state.min_state, low, period - 1, (period, period - 1));
        _ = calc_max(&mut state.max_state, high, period - 1, (period, period - 1));
        state
    }
}
/// Returns information about the Aroon indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Aroon indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "aroon",
        full_name: "Aroon",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["aroon_down", "aroon_up"],
        optional_outputs: &[],
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Aroon indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Aroon calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the Aroon indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Aroon calculation.
///
/// # Returns
///
/// The number of output values produced by the Aroon calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Aroon indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; Aroon has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `aroon_down`, `outputs[1]` is `aroon_up`,
/// and `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let period = options[0] as usize;
    let multiplier = multiplier(period);
    let high = inputs[0];
    let low = inputs[1];

    /*let (mut aroon_up_line, mut aroon_down_line) = {
        let capacity = output_length(high.len(), options);
        (crate::uninit_vec!(f64, capacity), crate::uninit_vec!(f64, capacity))
    };*/
    let (mut aroon_up_line, mut aroon_down_line) = {
        let capacity = output_length(high.len(), options);
        (vec![0.0; capacity], vec![0.0; capacity])
    };
    //let mut aroon_up_line = vec![0.0; capacity];
    //let mut aroon_down_line = vec![0.0; capacity];
    let mut state = State::new(low[0], period, high[0], period);
    match period {
        1..=4 => {
            cycle_aroon::<1>(
                (high, low),
                period,
                multiplier,
                (&mut aroon_down_line, &mut aroon_up_line),
                &mut state,
            );
        }
        5..30 => {
            cycle_aroon::<4>(
                (high, low),
                period,
                multiplier,
                (&mut aroon_down_line, &mut aroon_up_line),
                &mut state,
            );
        }
        _ => {
            cycle_aroon::<8>(
                (high, low),
                period,
                multiplier,
                (&mut aroon_down_line, &mut aroon_up_line),
                &mut state,
            );
        }
    }
    Ok((
        vec![aroon_down_line, aroon_up_line],
        IndicatorState::new(high, low, state, period, multiplier),
    ))
}

/// Performs the main calculation loop for the Aroon indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high and low price slices.
/// * `period` - The period for the Aroon calculation.
/// * `multiplier` - The multiplier used to scale Aroon values (100 / period).
/// * `output_lines` - A tuple of mutable slices for storing the Aroon down and Aroon up lines.
/// * `state` - A mutable reference to the current indicator state.
fn cycle_aroon<const N: usize>(
    inputs: (&[f64], &[f64]),
    period: usize,
    multiplier: f64,
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State,
) {
    //let mut count = 0;
    let (aroon_down_line, aroon_up_line) = output_lines;
    for (j, i) in (period..inputs.0.len()).enumerate() {
        unsafe {
            (
                *aroon_down_line.get_unchecked_mut(j),
                *aroon_up_line.get_unchecked_mut(j),
            ) = calc_unchecked::<N>(state, inputs, i, period, multiplier);
        }
    }
    //println!("Regular SEARCH COUNT: {:?}, period: {:?}", count, period);
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64]),
    i: usize,
    period: usize,
    multiplier: f64,
) -> (f64, f64) {
    let (high, low) = inputs;
    let (_, min_trail) = calc_min(&mut state.min_state, low, i, (period, period));
    let (_, max_trail) = calc_max(&mut state.max_state, high, i, (period, period));

    let aroon_up = (period - max_trail) as f64 * multiplier;
    let aroon_down = (period - min_trail) as f64 * multiplier;
    (aroon_down, aroon_up)
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    inputs: (&[f64], &[f64]),
    i: usize,
    period: usize,
    multiplier: f64,
) -> (f64, f64) {
    let (high, low) = inputs;
    let (_, min_trail) = calc_min_unchecked::<N>(&mut state.min_state, low, i, (period, period));
    let (_, max_trail) = calc_max_unchecked::<N>(&mut state.max_state, high, i, (period, period));

    let aroon_up = (period - max_trail) as f64 * multiplier;
    let aroon_down = (period - min_trail) as f64 * multiplier;
    (aroon_down, aroon_up)
}

pub fn multiplier(period: usize) -> f64 {
    100.0 / period as f64
}
