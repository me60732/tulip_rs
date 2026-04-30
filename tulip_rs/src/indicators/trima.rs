use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options as indicator;
}

/// Provides metadata about the Triangular Moving Average (TRIMA) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "trima",
        full_name: "Triangular Moving Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trima"],
        optional_outputs: &[],
    }
}

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
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum number of data points required by TRIMA before it can produce output.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize
}

/// Computes the final output length based on the data length, options, and optional recent-only calculations.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

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
/// * `real` - Sliced input data.
/// * `period` - The period for TRIMA.
/// * `start` - The index offset: we begin calculations from `start` inclusive.
/// * `trima_line` - Storage for final TRIMA values.
/// * `weight_sum` - Accumulated weighted sum of data points.
/// * `lead_sum` - Accumulated sum for the "leading" portion.
/// * `trail_sum` - Accumulated sum for the "trailing" portion.
///
/// # Returns
///
/// A tuple of updated rolling sums `(weight_sum, lead_sum, trail_sum)`.
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
                real.get_unchecked(j),//tsi2),
                multiplier,
            );
        }

        (lsi, tsi1) = (lsi + 1, tsi1 + 1);
    }
}
/// Calculates the Triangular Moving Average (TRIMA) output for one iteration.
///
/// # Arguments
///
/// * `real` - A slice of the input data (e.g., price series).
/// * `i` - The current index in the data slice.
/// * `lsi`, `tsi1`, `tsi2` - Offsets for lead and trail sums.
/// * `weight_sum` - Accumulated weighted sum of data points.
/// * `lead_sum` - Accumulated sum for the "leading" portion.
/// * `trail_sum` - Accumulated sum for the "trailing" portion.
/// * `multiplier` - Normalization factor, typically `1.0 / denominator`.
///
/// # Returns
///
/// A tuple containing:
/// 1. `trima`: The current TRIMA value.
/// 2. `weight_sum`: Updated weighted sum of data points.
/// 3. `lead_sum`: Updated leading sum.
/// 4. `trail_sum`: Updated trailing sum.
///
/// The logic updates these rolling sums in one pass to compute TRIMA.
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
