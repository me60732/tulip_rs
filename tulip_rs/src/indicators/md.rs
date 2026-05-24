use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::sma::calc as sma_calc;
pub use crate::indicators::sma::{init_state, multiplier};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::md_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::md_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::md_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::md_simd::indicator_by_options as indicator;
}

use std::simd::{num::SimdFloat, Simd};
/// Returns information about the Mean Deviation (MD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MD indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "md",
        display_type: DisplayType::Math,
        indicator_type: IndicatorType::Volatility,
        full_name: "Mean Deviation",
        inputs: &["real"],
        options: &["period"],
        outputs: &["md"],
        optional_outputs: &["sma"],
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    multiplier: f64,
    sum: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], sum: f64, multiplier: f64, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            multiplier,
            sum,
            period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        // Calculate capacities
        self.real.extend_from_slice(inputs[0]);

        let (mut md_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        self.sum = cycle_md(
            &self.real,
            self.sum,
            self.period,
            self.multiplier,
            &mut md_line,
            &mut sma_line,
        );

        self.real.drain(..self.real.len() - self.period);
        Ok(vec![md_line, sma_line])
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
/// Returns the minimum amount of data required for the MD indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the MD calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the MD indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the MD calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Mean Deviation (MD) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to enable the optional output
///   (`sma`); `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `md`
/// - `outputs[1]` — `sma` (empty if not requested)
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    if options[0] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut sum = init_state(real, period);
    let (mut md_line, mut sma_line) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                sma_line: capacity
            ),
        )
    };

    sum = cycle_md(real, sum, period, multiplier, &mut md_line, &mut sma_line);

    Ok((
        vec![md_line, sma_line],
        IndicatorState::new(real, sum, multiplier, period),
    ))
}

/// Performs the main calculation loop for the MD indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `sum` - The running sum for the SMA calculation.
/// * `period` - The period for the MD calculation.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
/// * `md_line` - A mutable slice for storing the MD output values.
/// * `sma_line` - A mutable slice for storing optional SMA output values.
///
/// # Returns
///
/// The updated running sum.
fn cycle_md(
    real: &[f64],
    mut sum: f64,
    period: usize,
    multiplier: f64,
    md_line: &mut [f64],
    sma_line: &mut [f64],
) -> f64 {
    let (want_sma, _) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (value, prev_value, prev_slice) = unsafe {
            (
                real.get_unchecked(i),
                real.get_unchecked(i - period),
                real.get_unchecked(i + 1 - period..=i),
            )
        };

        let (md, sma) = calc_simd::<4>(value, prev_value, prev_slice, &mut sum, multiplier);
        unsafe { *md_line.get_unchecked_mut(j) = md };

        if want_sma {
            crate::store_optional_outputs!(j,
                want_sma, sma_line => sma
            );
        }
    }

    sum
}

/// Calculates the current Mean Deviation (MD) value.
///
/// # Arguments
///
/// * `value` - The current input value.
/// * `prev_value` - The value leaving the rolling window.
/// * `slice` - The current window of values used to compute the mean deviation.
/// * `sum` - A mutable reference to the running sum for the SMA.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
///
/// # Returns
///
/// A tuple containing the MD value and the current SMA value.
#[inline(always)]
pub fn calc(
    value: &f64,
    prev_value: &f64,
    slice: &[f64],
    sum: &mut f64,
    multiplier: f64,
) -> (f64, f64) {
    let sma = sma_calc(sum, value, prev_value, &multiplier);

    let mean_deviation = calc_md(slice, sma, multiplier);
    (mean_deviation, sma)
}
#[inline(always)]
pub fn calc_simd<const N: usize>(
    value: &f64,
    prev_value: &f64,
    slice: &[f64],
    sum: &mut f64,
    multiplier: f64,
) -> (f64, f64) {
    let sma = sma_calc(sum, value, prev_value, &multiplier);

    let mean_deviation = calc_md_simd::<N>(slice, sma, multiplier);
    (mean_deviation, sma)
}
#[inline(always)]
pub(crate) fn calc_md_simd<const N: usize>(slice: &[f64], sma: f64, multiplier: f64) -> f64 {
    //let mut abs_dev_sum = 0.0;
    let sma_vec = Simd::<f64, N>::splat(sma);

    // Process 4 elements at a time
    let chunks = slice.chunks_exact(N);
    let remainder = chunks.remainder();

    let mut sum_vec = Simd::splat(0.0);
    for chunk in chunks {
        let vals = Simd::from_slice(chunk);
        //let abs_diff = (vals - sma_vec).abs();
        sum_vec += (vals - sma_vec).abs();
        //sum_vec += abs_diff;
    }

    // Sum the SIMD register
    //let mut abs_dev_sum = sum_vec.to_array().iter().sum::<f64>();
    let mut abs_dev_sum = sum_vec.reduce_sum();
    // Handle remainder
    for &x in remainder {
        abs_dev_sum += (x - sma).abs();
    }

    let md = abs_dev_sum * multiplier;
    md.max(f64::EPSILON)
}
#[inline(always)]
pub(crate) fn calc_md(real: &[f64], sma: f64, multiplier: f64) -> f64 {
    let md = real.iter().map(|&x| (x - sma).abs()).sum::<f64>() * multiplier;
    md.max(f64::EPSILON)
}
