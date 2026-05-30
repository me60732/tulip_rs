use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_options as indicator;
}

pub const INFO: Info = Info {
    name: "vwma",
    full_name: "Volume Weighted Moving Average",
    indicator_type: IndicatorType::Trend,
    // Two inputs: close and volume.
    inputs: &["close", "volume"],
    // One option: period.
    options: &["period"],
    outputs: &["vwma"],
    // No optional outputs.
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "vwma",
        label: "VWMA",
        display_type: DisplayType::Overlay,
        outputs: &["vwma"],
    }],
};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    close: Vec<f64>,
    volume: Vec<f64>,
    state: State,
    period: usize,
}
impl IndicatorState {
    pub fn new(close: &[f64], volume: &[f64], state: State, period: usize) -> Self {
        Self {
            close: close[close.len() - period..].to_vec(),
            volume: volume[volume.len() - period..].to_vec(),
            state,
            period,
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
        self.close.extend_from_slice(inputs[0]);
        self.volume.extend_from_slice(inputs[1]);

        let mut vwma_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            &self.close,
            &self.volume,
            self.period,
            &mut self.state,
            &mut vwma_line,
        );

        self.close.drain(..self.close.len() - self.period);
        self.volume.drain(..self.volume.len() - self.period);

        Ok(vec![vwma_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub sum: f64,
    pub vol_sum: f64,
}
impl State {
    pub fn new(sum: f64, vol_sum: f64) -> Self {
        State { sum, vol_sum }
    }
    /// Initializes VWMA by computing the initial numerator and denominator sums over the first period,
    /// then computing the first VWMA value.
    pub fn init_state(period: usize, close: &[f64], volume: &[f64]) -> Self {
        let mut sum = 0.0;
        let mut vol_sum = 0.0;
        for i in 0..period {
            sum += close[i] * volume[i];
            vol_sum += volume[i];
        }
        Self::new(sum, vol_sum)
    }
    #[inline(always)]
    pub fn calc(&mut self, values: (&f64, &f64), prev_values: (&f64, &f64)) -> f64 {
        let (close, volume) = values;
        let (prev_close, prev_volume) = prev_values;
        // Add new bar's contribution.
        self.sum += (close * volume) - (prev_close * prev_volume);
        self.vol_sum += volume - prev_volume;

        if self.vol_sum == 0.0 {
            return 0.0;
        }
        self.sum / self.vol_sum
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options: `[period]`.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the VWMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[period]`.
///
/// # Returns
///
/// The minimum amount of data required (period + 1).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the VWMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Volume Weighted Moving Average (VWMA) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `close` (close price series)
/// * `inputs[1]` — `volume`
///
/// # Options
///
/// * `options[0]` — `period`
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `vwma` and `state`
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
    let close = inputs[0];
    let volume = inputs[1];

    let mut vwma_line = {
        let capacity = output_length(close.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    // Initialize state.
    let mut state = State::init_state(period, close, volume);

    // Process from index = period (first full window is available).
    cycle(close, volume, period, &mut state, &mut vwma_line);

    Ok((
        vec![vwma_line],
        IndicatorState::new(close, volume, state, period),
    ))
}

/// Iterates over the close and volume arrays and writes VWMA values into `vwma_line`.
///
/// # Arguments
///
/// * `close` - The full close price input slice.
/// * `volume` - The full volume input slice.
/// * `period` - The period for the VWMA calculation.
/// * `state` - Mutable reference to the rolling `State` (weighted sum and volume sum).
/// * `vwma_line` - Mutable output slice for VWMA values.
fn cycle(close: &[f64], volume: &[f64], period: usize, state: &mut State, vwma_line: &mut [f64]) {
    for (j, i) in (period..close.len()).enumerate() {
        unsafe {
            *vwma_line.get_unchecked_mut(j) = state.calc(
                (close.get_unchecked(i), volume.get_unchecked(i)),
                (close.get_unchecked(j), volume.get_unchecked(j)),
            )
        };
    }
}

/// Calculates a single VWMA value for one bar, updating the rolling state in place.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State` (weighted sum and volume sum).
/// * `values` - A tuple of `(close, volume)` for the current bar.
/// * `prev_values` - A tuple of `(prev_close, prev_volume)` for the bar leaving the window.
///
/// # Returns
///
/// The VWMA value for this bar.
#[inline(always)]
pub fn calc(state: &mut State, values: (&f64, &f64), prev_values: (&f64, &f64)) -> f64 {
    state.calc(values, prev_values)
}
