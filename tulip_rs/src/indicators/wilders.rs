use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::wilders_simd::indicator_by_options as indicator;
}

/// Returns information about the Wilder's Smoothing (WILDERS) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the WILDERS indicator.
pub const INFO: Info = Info {
    name: "wilders",
    full_name: "Wilder's Smoothing",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period"],
    outputs: &["wilders"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "wilders",
        label: "WILDERS",
        display_type: DisplayType::Overlay,
        outputs: &["wilders"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub multipliers: (f64, f64),
    pub wilders: f64,
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
        self.wilders = self
            .wilders
            .mul_add(self.multipliers.0, value * self.multipliers.1);
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
/// * `options` - A slice containing the indicator options: `[period]`.
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
/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the WILDERS calculation.
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
/// Calculates the Wilder's Smoothing (WILDERS) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `real` (price series)
///
/// # Options
///
/// * `options[0]` — `period`
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `wilders` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
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
    let (mut wilders, multipliers) = init_state(inputs[0], period);

    let real = &inputs[0][period..];
    for i in 0..real.len() {
        unsafe {
            wilders = calc(wilders, *real.get_unchecked(i), multipliers);
            *wilders_line.get_unchecked_mut(i) = wilders;
        }
    }

    Ok((vec![wilders_line], IndicatorState::new(wilders, multipliers)))
}

/// Calculates the current value of Wilder's Smoothing for a single step.
///
/// # Arguments
///
/// * `prev_wilders` - The previous WILDERS value.
/// * `value` - The current input value.
/// * `multiplier` - The decay multiplier `((period - 1) / period)` from `multiplier()`.
///
/// # Returns
///
/// The updated WILDERS value.
#[inline(always)]
pub fn calc(prev_wilders: f64, value: f64, multipliers: (f64, f64)) -> f64 {
    prev_wilders.mul_add(multipliers.0, value * multipliers.1)
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
