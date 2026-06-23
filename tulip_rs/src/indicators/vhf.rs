use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::max::{State as MaxState, CHUNK_1, CHUNK_4};
use crate::indicators::min::State as MinState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vhf_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
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

        if CHUNK_1.contains(&self.period) {
            cycle::<1>(&self.real, self.period, &mut self.state, &mut vhf_line);
        } else if CHUNK_4.contains(&self.period) {
            cycle::<4>(&self.real, self.period, &mut self.state, &mut vhf_line);
        } else {
            cycle::<8>(&self.real, self.period, &mut self.state, &mut vhf_line);
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
    pub fn init_state(real: &[f64], period: usize, indicator_line: &mut [f64]) -> Self {
        let mut state = State::new((real[0], period), (real[0], period), 0.0);
    
        for i in 1..=period {
            state.sum += (real[i] - real[i - 1]).abs();
        }
        let (min, _) = state.min_state.calc(real, period, (period, period - 1));
        let (max, _) = state.max_state.calc(real, period, (period, period - 1));
        let vhf = (max - min) / state.sum.max(f64::EPSILON);
        indicator_line[0] = vhf;
        state
    }
    /// Calculates a single VHF value from the current state.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the current indicator state.
    /// * `values` - A tuple `(value, prev_real, old_real, drop_real)` of price references used to update the rolling sum.
    /// * `real` - The full input data slice (used for min/max lookups).
    /// * `periods` - A tuple `(period, period - 1)` used for min/max window calculations.
    /// * `i` - The current index in `real`.
    ///
    /// # Returns
    ///
    /// The current VHF value.
    #[inline(always)]
    pub fn calc(
        &mut self,
        values: (&f64, &f64, &f64, &f64),
        real: &[f64],
        periods: (usize, usize),
        i: usize,
    ) -> f64 {
        let (value, prev_real, old_real, drop_real) = values;
        self.sum += (value - prev_real).abs() - (old_real - drop_real).abs();
    
        let (min, _) = self.min_state.calc(real, i, periods);
        let (max, _) = self.max_state.calc(real, i, periods);
    
        (max - min) / self.sum.max(f64::EPSILON)
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked<const N: usize>(
        &mut self,
        values: (&f64, &f64, &f64, &f64),
        real: &[f64],
        periods: (usize, usize),
        i: usize,
    ) -> f64 {
        let (value, prev_real, old_real, drop_real) = values;
        self.sum += (value - prev_real).abs() - (old_real - drop_real).abs();
    
        let (min, _) = self.min_state.calc_unchecked::<N>(real, i, periods);
        let (max, _) = self.max_state.calc_unchecked::<N>(real, i, periods);
        (max - min) / self.sum.max(f64::EPSILON)
    }
}
/// Returns information about the Vertical Horizontal Filter (VHF) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the VHF indicator.
pub const INFO: Info = Info {
    name: "vhf",
    full_name: "Vertical Horizontal Filter",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period"],
    outputs: &["vhf"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "vhf",
        label: "VHF",
        display_type: DisplayType::Indicator,
        outputs: &["vhf"],
    }],
};
/// Returns the minimum amount of data required for the VHF indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the VHF calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the VHF calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Vertical Horizontal Filter (VHF) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (price series)
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; VHF has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `vhf` and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
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

    let mut state = State::init_state(real, period, &mut vhf_line);

    if CHUNK_1.contains(&period) {
        cycle::<1>(real, period, &mut state, &mut vhf_line[1..]);
    } else if CHUNK_4.contains(&period) {
        cycle::<4>(real, period, &mut state, &mut vhf_line[1..]);
    } else {
        cycle::<8>(real, period, &mut state, &mut vhf_line[1..]);
    }

    Ok((vec![vhf_line], IndicatorState::new(state, real, period)))
}


/// Performs the main calculation loop for the VHF indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the VHF calculation.
/// * `state` - A mutable reference to the current indicator state.
/// * `indicator_line` - A mutable slice for storing the VHF output values.
fn cycle<const N: usize>(
    real: &[f64],
    period: usize,
    state: &mut State,
    indicator_line: &mut [f64],
) {
    let periods = (period, period - 1);

    for (j, i) in (period + 1..real.len()).enumerate() {
        unsafe {
            *indicator_line.get_unchecked_mut(j) = state.calc_unchecked::<N>(
                (
                    real.get_unchecked(i),
                    real.get_unchecked(i - 1),
                    real.get_unchecked(j + 1), //i - period),
                    real.get_unchecked(j),     //i - period - 1),
                ),
                real,
                periods,
                i,
            );
        }
    }
}

