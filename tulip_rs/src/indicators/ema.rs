use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;

use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};
//use wide::*;

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::ema_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: (f64, f64),
    ema: f64,
}
impl IndicatorState {
    pub fn new(ema: f64, multipliers: (f64, f64)) -> Self {
        Self { ema, multipliers }
    }
    pub fn get_ema(&self) -> f64 {
        self.ema
    }
    pub fn get_multipliers(&self) -> (f64, f64) {
        self.multipliers
    }
    pub fn set_ema(&mut self, ema: f64) {
        self.ema = ema;
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real: &[f64] = inputs[0];
        let mut ema_line = crate::uninit_vec!(f64, real.len());
        //let mut ema_line = vec![0.0; real.len()];
        //self.ema = cycle_ema(real, self.multiplier, self.ema, 0, &mut ema_line);
        for (j, i) in (0..real.len()).enumerate() {
            unsafe {
                self.ema = calc(real.get_unchecked(i), self.ema, self.multipliers);
                *ema_line.get_unchecked_mut(j) = self.ema;
            }
        }
        Ok(vec![ema_line])
    }
}
/// Returns information about the Exponential Moving Average (EMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the EMA indicator.
pub const INFO: Info = Info {
    name: "ema",
    full_name: "Exponential Moving Average",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period"],
    outputs: &["ema"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "ema",
        label: "EMA",
        display_type: DisplayType::Overlay,
        outputs: &["ema"],
    }],
};
/// Returns the number of output values produced by the EMA indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the EMA calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the smoothing
/// multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Info(INFO),
        min_data,
    )
}
/// Returns the minimum amount of data required for the EMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the EMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Initializes the EMA state by computing the initial EMA value over the first `period` elements.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `period` - The EMA period.
/// * `multipliers` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
///
/// # Returns
///
/// The initial EMA value after processing the first `period` elements.
pub fn init_state(real: &[f64], period: usize, multipliers: (f64, f64)) -> f64 {
    let mut ema = real[0];
    for i in 1..period {
        ema = calc(&real[i], ema, multipliers);
    }
    ema
}

/// Calculates the Exponential Moving Average (EMA) indicator for an entire dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Outputs
///
/// * `outputs[0]` — `ema` line
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; EMA has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `ema` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);

    validate_inputs(inputs, min_data(options))?;

    let mut ema = init_state(inputs[0], period, multipliers);
    let real = &inputs[0][period..];
    let mut ema_line = {
        let capacity = output_length(inputs[0].len(), &[period as f64]);
        crate::uninit_vec!(f64, capacity)
    };

    for i in 0..real.len() {
        unsafe {
            ema = calc(real.get_unchecked(i), ema, multipliers);
            *ema_line.get_unchecked_mut(i) = ema;
        }
    }
    Ok((vec![ema_line], IndicatorState { ema, multipliers }))
}

#[inline(always)]
pub fn calc(value: &f64, prev_ema: f64, multipliers: (f64, f64)) -> f64 {
    let (multiplier, inv_multiplier) = multipliers;

    //prev_ema * inv_multiplier + value * multiplier
    prev_ema.mul_add(inv_multiplier, value * multiplier)
}

///partial calc for batch calculating ema, correct final result by * multiplier; multiplier(period).0
#[inline(always)]
pub fn partial_calc(value: f64, prev_ema: f64, inv_multiplier: f64) -> f64 {
    //prev_ema * inv_multiplier + value // Missing the `* multiplier` part
    prev_ema.mul_add(inv_multiplier, value)
}
#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64) {
    let per = 2.0 / (period as f64 + 1.0);
    (per, 1.0 - per)
}
