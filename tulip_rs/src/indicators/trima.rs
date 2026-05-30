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
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options as indicator;
}

/// Returns information about the Triangular Moving Average (TRIMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TRIMA indicator.
pub const INFO: Info = Info {
    name: "trima",
    full_name: "Triangular Moving Average",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["period"],
    outputs: &["trima"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "trima",
        label: "TRIMA",
        display_type: DisplayType::Overlay,
        outputs: &["trima"],
    }],
};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State,
    multiplier: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State, multiplier: f64, period: usize) -> Self {
        Self {
            real: real[real.len() - period + 1..].to_vec(),
            state,
            multiplier,
            period,
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

        let mut trima_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_trima(
            &self.real,
            self.period,
            self.multiplier,
            &mut trima_line,
            &mut self.state,
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![trima_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub weight_sum: f64,
    pub lead_sum: f64,
    pub trail_sum: f64,
}
impl State {
    pub fn new(weight_sum: f64, lead_sum: f64, trail_sum: f64) -> Self {
        Self {
            weight_sum,
            lead_sum,
            trail_sum,
        }
    }
    /// Calculates the initial sums (`weight_sum`, `lead_sum`, `trail_sum`) so the iteration can start properly.
    ///
    /// # Arguments
    ///
    /// * `real` - A slice of the input data (e.g., price series).
    /// * `period` - The TRIMA period.
    ///
    /// # Returns
    ///
    /// `(weight_sum, lead_sum, trail_sum)`:
    /// - `weight_sum`: Accumulated weighted sum for the first (period-1) elements.
    /// - `lead_sum`: Accumulated sum of the 'lead' portion.
    /// - `trail_sum`: Accumulated sum of the 'trail' portion.
    ///
    /// This is used to "warm up" the rolling sums before iterating through the rest of the data.
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let mut weight_sum = 0.0;
        let mut lead_sum = 0.0;
        let mut trail_sum = 0.0;
        let mut w = 1.0;

        let (lead_period, trail_period) = initialize_periods(period);

        for (i, &value) in real.iter().enumerate().take(period - 1) {
            weight_sum += value * w;
            if i + 1 > period - lead_period {
                lead_sum += value;
            }
            if i < trail_period {
                trail_sum += value;
            }
            if i + 1 < trail_period {
                w += 1.0;
            }
            if i + 1 >= period - lead_period {
                w -= 1.0;
            }
        }
        Self::new(weight_sum, lead_sum, trail_sum)
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
/// Returns the minimum amount of data required for the TRIMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the TRIMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TRIMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Triangular Moving Average (TRIMA) indicator over the full input dataset.
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
/// * `_optional_outputs` - Unused; TRIMA has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `trima` and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
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
    let real = inputs[0];

    let mut trima_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    // Initialize rolling sums for the 2 SMA passes in TRIMA.
    // The original TRIMA logic can be performed with a single pass using these sums.
    let mut state = State::init_state(real, period);

    cycle_trima(real, period, multiplier, &mut trima_line, &mut state);

    Ok((
        vec![trima_line],
        IndicatorState::new(real, state, multiplier, period),
    ))
}

/// Performs the main calculation loop for the TRIMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the TRIMA calculation.
/// * `multiplier` - Normalization factor applied to produce the final TRIMA value.
/// * `trima_line` - A mutable slice for storing the TRIMA output values.
/// * `state` - A mutable reference to the rolling sums state.
pub fn cycle_trima(
    real: &[f64],
    period: usize,
    multiplier: f64,
    trima_line: &mut [f64],
    state: &mut State,
) {
    let (mut lsi, mut tsi1) = initialize_counters(period);

    for (j, i) in (period - 1..real.len()).enumerate() {
        unsafe {
            *trima_line.get_unchecked_mut(j) = calc(
                state,
                real.get_unchecked(i),
                real.get_unchecked(lsi),
                real.get_unchecked(tsi1),
                real.get_unchecked(j), //tsi2),
                multiplier,
            );
        }

        (lsi, tsi1) = (lsi + 1, tsi1 + 1);
    }
}
/// Calculates the Triangular Moving Average (TRIMA) output for one iteration,
/// updating the rolling sums in place.
///
/// # Arguments
///
/// * `state` - A mutable reference to the rolling sums state (`weight_sum`, `lead_sum`, `trail_sum`).
/// * `real` - The current input value (e.g., current price).
/// * `lsi` - The value being removed from the lead sum.
/// * `tsi1` - The value being added to the trail sum.
/// * `tsi2` - The value being removed from the trail sum.
/// * `multiplier` - Normalization factor, typically `1.0 / denominator`.
///
/// # Returns
///
/// The current TRIMA value.
#[inline(always)]
pub fn calc(
    state: &mut State,
    real: &f64,
    lsi: &f64,
    tsi1: &f64,
    tsi2: &f64,
    multiplier: f64,
) -> f64 {
    let (mut weight_sum, mut lead_sum, mut trail_sum) =
        (state.weight_sum, state.lead_sum, state.trail_sum);
    weight_sum += real;
    let trima = weight_sum * multiplier;
    lead_sum += real;
    weight_sum += lead_sum - trail_sum;
    lead_sum -= lsi;
    trail_sum += tsi1 - tsi2;

    (state.weight_sum, state.lead_sum, state.trail_sum) = (weight_sum, lead_sum, trail_sum);
    trima
}
/// Determines the 'lead' and 'trail' periods used for TRIMA calculations.
///
/// A Triangular Moving Average splits its period roughly in half, so:
/// - If `period` is odd, `lead_period` is simply `period / 2`.
/// - If `period` is even, `lead_period` becomes `(period / 2) - 1`.
///
/// The `trail_period` is always one more than `lead_period`.
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// `(lead_period, trail_period)`:
/// - `lead_period`: The number of values considered as the 'lead' half.
/// - `trail_period`: The number of values for the trailing half.
#[inline(always)]
fn initialize_periods(period: usize) -> (usize, usize) {
    let lead_period = if period % 2 == 1 {
        period / 2
    } else {
        period / 2 - 1
    };
    let trail_period = lead_period + 1;
    (lead_period, trail_period)
}
/// Calculates the offset indices needed for lead and trail lookups used in the iteration.
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// `(lsi, tsi1, tsi2)`:
/// - `lsi`: How far back we remove from the lead sum.
/// - `tsi1`: How far back we add to the trail sum.
/// - `tsi2`: How far back we remove from the trail sum.
#[inline(always)]
pub fn initialize_counters(period: usize) -> (usize, usize) {
    let (lead_period, trail_period) = initialize_periods(period);
    let lsi = (period - 1) - lead_period + 1;
    let tsi1 = trail_period;
    (lsi, tsi1)
}

/// Computes a multiplier for normalizing the weighted sums in the TRIMA calculation.
///
/// If the period is odd:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2 + 1))`
/// If the period is even:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2))`
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// A `f64` scaling factor applied to produce the final TRIMA value.
pub fn multiplier(period: usize) -> f64 {
    if period % 2 == 1 {
        1.0 / ((period / 2 + 1) * (period / 2 + 1)) as f64
    } else {
        1.0 / ((period / 2 + 1) * (period / 2)) as f64
    }
}
