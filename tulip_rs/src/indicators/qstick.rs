use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::sma::multiplier;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

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
    period: usize,
    sum: f64,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(open: &[f64], close: &[f64], sum: f64, period: usize, multiplier: f64) -> Self {
        Self {
            open: open[open.len() - period..].to_vec(),
            close: close[close.len() - period..].to_vec(),
            sum,
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

        self.open.extend_from_slice(inputs[0]);
        self.close.extend_from_slice(inputs[1]);

        let mut qstick_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        self.sum = cycle_qstick(
            &self.open,
            &self.close,
            self.period,
            self.multiplier,
            &mut qstick_line,
            self.sum,
        );

        self.close.drain(..self.close.len() - self.period);
        self.open.drain(..self.open.len() - self.period);

        Ok(vec![qstick_line])
    }
}

/// Returns information about the QStick indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the QStick indicator.
pub const INFO: Info = Info {
    name: "qstick",
    full_name: "QStick",
    indicator_type: IndicatorType::Momentum,
    inputs: &["open", "close"],
    options: &["period"],
    outputs: &["qstick"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "qstick",
        label: "QSTICK",
        display_type: DisplayType::Indicator,
        outputs: &["qstick"],
    }],
};
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
/// Returns the minimum amount of data required for the QStick indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the QStick calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}
/// Calculates the output length for the QStick indicator given the input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the QStick calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the QStick indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — open prices
/// * `inputs[1]` — close prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `qstick`
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let open = inputs[0];
    let close = inputs[1];

    let mut qstick_line = {
        let capacity = output_length(open.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut sum = init(open, close, period);
    sum = cycle_qstick(open, close, period, multiplier, &mut qstick_line, sum);

    Ok((
        vec![qstick_line],
        IndicatorState::new(open, close, sum, period, multiplier),
    ))
}
#[inline(always)]
pub fn init(open: &[f64], close: &[f64], period: usize) -> f64 {
    let mut sum = 0.0;
    for i in 0..period {
        sum += close[i] - open[i];
    }
    sum
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
    multiplier: f64,
    qstick_line: &mut [f64],
    mut sum: f64,
) -> f64 {
    for (j, i) in (period..open.len()).enumerate() {
        unsafe {
            *qstick_line.get_unchecked_mut(j) = calc(
                *open.get_unchecked(i),
                *close.get_unchecked(i),
                *open.get_unchecked(j),
                *close.get_unchecked(j),
                &mut sum,
                multiplier,
            );
        }
    }

    sum
}
/// Calculates the QStick value for a single bar of data.
///
/// # Arguments
///
/// * `open` - The open price for the current bar.
/// * `close` - The close price for the current bar.
/// * `prev_open` - The open price for the previous bar.
/// * `prev_close` - The close price for the previous bar.
/// * `sum` - The current sum of the differences between close and open prices.
/// * `multiplier` - The multiplier for the QStick calculation.
///
/// # Returns
///
/// The QStick value for the current bar.
#[inline(always)]
pub fn calc(
    open: f64,
    close: f64,
    prev_open: f64,
    prev_close: f64,
    sum: &mut f64,
    multiplier: f64,
) -> f64 {
    let mut s = *sum;
    s += (close - open) - (prev_close - prev_open);
    *sum = s;
    s * multiplier
}
