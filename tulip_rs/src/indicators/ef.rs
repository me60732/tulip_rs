use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ef_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ef_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ef_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::ef_simd::indicator_by_options as indicator;
}
/// Returns information about the Kaufman's Efficiency Ratio (EF) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the EF indicator.
pub const INFO: Info = Info {
    name: "ef",
    indicator_type: IndicatorType::Trend,
    full_name: "Efficiency Ratio",
    inputs: &["real"],
    options: &["period"],
    outputs: &["ef"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "ef",
        label: "EF",
        display_type: DisplayType::Indicator,
        outputs: &["ef"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
    sum: f64,
}
impl IndicatorState {
    /// Creates a new `IndicatorState` for streaming continuation.
    ///
    /// # Arguments
    ///
    /// * `real` - The full real price slice from the just-completed batch.
    /// * `sum` - The current rolling sum of absolute price changes (carried forward).
    /// * `period` - The EF lookback period.
    pub fn new(real: &[f64], sum: f64, period: usize) -> Self {
        Self {
            period,
            sum,
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

        let mut ef_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        cycle_ef(&self.real, &mut self.sum, self.period, &mut ef_line);
        self.real.drain(..self.real.len() - self.period - 1);

        Ok(vec![ef_line])
    }
}

/// Initialises the EF state and writes the first output value.
///
/// Computes the initial rolling sum of absolute changes over `[1, period]` and
/// uses it to calculate the very first EF value, which is stored in `ef_line[0]`.
///
/// # Arguments
///
/// * `real` - Input price slice; must contain at least `period + 1` elements.
/// * `period` - EF lookback period.
/// * `ef_line` - Output buffer; element `[0]` is written by this call.
///
/// # Returns
///
/// The initial rolling sum of absolute price changes, ready to be passed to
/// subsequent calls to [`cycle_ef`] or [`calc`].
pub fn init(real: &[f64], period: usize, ef_line: &mut [f64]) -> f64 {
    let mut sum = (1..period).map(|i| (real[i] - real[i - 1]).abs()).sum();
    let values = unsafe {
        (
            real.get_unchecked(period),
            real.get_unchecked(period - 1),
            real.get_unchecked(0),
            &0.0,
        )
    };
    let ef = calc(&mut sum, values, period, period);
    ef_line[0] = ef;

    sum
}

/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// EF is a bounded ratio with no exponential seed decay, so accuracy is
/// independent of `decimals`. This always returns the same value as
/// [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `_decimals` - Unused; accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars needed, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the EF indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the EF calculation.
///
/// # Returns
///
/// The minimum amount of data required (`period + 1`).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Returns the number of output values produced by the EF indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the EF calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Efficiency Ratio (EF) indicator for an entire dataset.
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
/// * `outputs[0]` — `ef` line (values in the range \[0.0, 1.0\])
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; EF has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `ef` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut ef_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut sum = init(real, period, &mut ef_line);
    cycle_ef(real, &mut sum, period, &mut ef_line[1..]);

    Ok((vec![ef_line], IndicatorState::new(real, sum, period)))
}

/// Performs the main calculation loop for the EF indicator.
///
/// # Arguments
///
/// * `real` - Full price slice including the lookback window at the front.
///   The loop iterates `(period+1)..real.len()`, so the number of outputs written
///   equals `real.len() - period - 1`; `ef_line` must be at least that long.
/// * `sum` - Mutable reference to the rolling absolute-movement accumulator.
/// * `period` - The EF lookback period.
/// * `ef_line` - Output buffer; receives one value per loop iteration (written
///   starting at index 0).
fn cycle_ef(real: &[f64], sum: &mut f64, period: usize, ef_line: &mut [f64]) {
    //real.iter().enumerate().skip(start).for_each(|(i, value)| {
    for (j, i) in (period + 1..real.len()).enumerate() {
        let values = unsafe {
            (
                real.get_unchecked(i),
                real.get_unchecked(i - 1),
                real.get_unchecked(j + 1),
                real.get_unchecked(j),
            )
        };
        let ef = calc(sum, values, period, i);

        unsafe { *ef_line.get_unchecked_mut(j) = ef };
    }
}

/// Computes Kaufman's Efficiency Ratio (ER), defined as:
///
/// ```text
/// ER = |price[t] - price[t-n]| / sum(|Δprice[i]|)
/// ```
///
/// The numerator measures the net directional movement over the lookback
/// window, while the denominator measures the total absolute movement (noise).
/// ER therefore ranges from:
///
///   • 1.0 → perfectly efficient trend (straight-line movement)
///   • 0.0 → completely inefficient movement (pure noise or no movement)
///
/// If the denominator is zero—meaning price did not move at all, or every
/// up‑move was exactly cancelled by a down‑move—the ER is defined as **0.0**.
/// This reflects a regime of maximum noise and zero trend efficiency.
///
/// ER must **not** be forced to 1.0 in this case.  Treating "no movement" as a
/// "perfect trend" would invert the intended behaviour of adaptive indicators
/// such as KAMA, which rely on ER to slow down during noisy or stagnant
/// conditions and speed up only when a genuine trend is present.
///
/// # Arguments
///
/// * `sum` - Mutable accumulator for the rolling sum of absolute price changes.
///   Updated in-place on every call.
/// * `values` - A tuple `(value, prev_value, last_value, old_value)` where:
///   - `value` — close price at bar `i` (current bar)
///   - `prev_value` — close price at bar `i - 1` (previous bar, used to update the rolling sum)
///   - `last_value` — close price at bar `i - period` (left edge of lookback; numerator anchor)
///   - `old_value` — close price at bar `i - period - 1` (expiring bar, removed from rolling sum)
/// * `period` - EF lookback period.
/// * `i` - Current bar index (determines whether the expiring bar should be removed).
///
/// # Returns
///
/// The Efficiency Ratio for the current bar, in the range `[0.0, 1.0]`.
#[inline(always)]
pub fn calc(sum: &mut f64, values: (&f64, &f64, &f64, &f64), period: usize, i: usize) -> f64 {
    let mut s = *sum;
    let (value, prev_value, last_value, old_value) = values;
    s += (value - prev_value).abs();
    if i > period {
        s -= (last_value - old_value).abs();
    }
    *sum = s;
    if s != 0.0 {
        (value - last_value).abs() / s
    } else {
        0.0
    }
}
