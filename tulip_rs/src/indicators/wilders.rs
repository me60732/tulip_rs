use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_options as indicator;
}

/// Returns information about the Wilder's Smoothing (WILDERS) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the WILDERS indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "wilders",
        full_name: "Wilder's Smoothing",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["wilders"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: (f64, f64),
    wilders: f64,
}
impl IndicatorState {
    pub fn new(wilders: f64, multipliers: (f64, f64)) -> Self {
        Self {
            multipliers,
            wilders,
        }
    }

    #[inline(always)]
    pub fn calc(&mut self, value: f64) -> f64 {
        self.wilders = self.wilders.mul_add(self.multipliers.0, value * self.multipliers.1);
        self.wilders
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real = inputs[0];
        let mut wilders_line = crate::uninit_vec!(f64, real.len());
        for i in 0..real.len() {
            unsafe { *wilders_line.get_unchecked_mut(i) = self.calc(*real.get_unchecked(i)) }
        }

        Ok(vec![wilders_line])
    }
}
/// Returns the minimum amount of data required for the WILDERS indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the WILDERS calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
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
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the WILDERS calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub fn init_state(real: &[f64], period: usize) -> (f64, (f64, f64)) {
    let wilders = real.iter().take(period).sum::<f64>() / period as f64;
    let multipliers = multiplier(period);
    (wilders, multipliers)
}
/// Calculates the Wilder's Smoothing (WILDERS) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the period for the WILDERS calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the WILDERS line.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    
    let mut wilders_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = {
        let (wilders, multipliers) = init_state(inputs[0], period);
        IndicatorState::new(wilders, multipliers)
    };
    let real = &inputs[0][period..];
    for i in 0..real.len() {
        unsafe { *wilders_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i)) }
    }

    Ok((vec![wilders_line], state))
}

/// Calculates the current value of the Wilder's Smoothing (WILDERS) indicator.
///
/// # Arguments
///
/// * `prev_wilders` - The previous WILDERS value.
/// * `period` - The period for the WILDERS calculation.
/// * `value` - The current input value.
///
/// # Returns
///
/// The current WILDERS value.
#[inline(always)]
pub fn calc(prev_wilders: f64, value: f64, multiplier: f64) -> f64 {
    //prev_wilders * multiplier + value * (1.0 - multiplier)
    prev_wilders.mul_add(multiplier, value * (1.0 - multiplier))
}

/*#[inline(always)]
pub fn calc1(prev_wilders: f64, value: f64, multipliers: (f64, f64)) -> f64 {
    prev_wilders * multipliers.0+ value * multipliers.1
}*/

#[inline(always)]
pub fn partial_calc(prev_wilders: f64, value: f64, multiplier: f64) -> f64 {
    //prev_wilders * multiplier + value
    prev_wilders.mul_add(multiplier, value)
}

/*#[inline(always)]
pub fn multiplier(period: usize) -> f64 {
    1.0 / period as f64
}*/
// returns dm_multiplier, inv_multiplier
#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64) {
    let multiplier = ((period - 1) as f64) / period as f64;
    (multiplier, 1.0 - multiplier)
}
