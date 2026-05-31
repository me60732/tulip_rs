use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub(crate) use crate::indicators::cmo::up_down;
pub use crate::indicators::wilders::multiplier;
use crate::indicators::wilders::calc_full as calc_wilders;
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
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options as indicator;
}

/// Returns information about the Relative Strength Index (RSI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the RSI indicator.
pub const INFO: Info = Info {
    name: "rsi",
    indicator_type: IndicatorType::Momentum,
    full_name: "Relative Strength Index",
    inputs: &["real"],
    options: &["period"],
    outputs: &["rsi"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        id: "rsi",
        label: "RSI",
        display_type: DisplayType::Indicator,
        outputs: &["rsi"],
    }],
};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64)) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut rsi_line = crate::uninit_vec!(f64, inputs[0].len());
        cycle_rsi(inputs[0], self.multipliers, &mut rsi_line, &mut self.state);

        Ok(vec![rsi_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub up_sum: f64,
    pub down_sum: f64,
    pub prev_real: f64,
}
impl State {
    pub fn new(prev_real: f64, up_sum: f64, down_sum: f64) -> Self {
        Self {
            prev_real,
            up_sum,
            down_sum,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let (up, down) = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        up_sum /= period as f64;
        down_sum /= period as f64;

        Self {
            up_sum,
            down_sum,
            prev_real: real[period],
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, cur_real: f64, multipliers: (f64, f64)) -> f64 {
        let (up, down) = up_down(cur_real, self.prev_real);

        self.up_sum = calc_wilders(self.up_sum, up, multipliers);
        self.down_sum = calc_wilders(self.down_sum, down, multipliers);
        
        self.prev_real = cur_real;

        100.0 * (self.up_sum / (self.up_sum + self.down_sum))
    }
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
/// Returns the minimum amount of data required for the RSI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the RSI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the RSI indicator given the input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the RSI calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options)
}

/// Calculates the Relative Strength Index (RSI) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (source) values
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
/// - `outputs[0]` — `rsi`
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
    let multipliers = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let mut rsi_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(inputs[0], period);

    cycle_rsi(
        &inputs[0][period + 1..],
        multipliers,
        &mut rsi_line,
        &mut state,
    );

    Ok((vec![rsi_line], IndicatorState { multipliers, state }))
}

/// Performs the main calculation loop for the RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `multiplier` - The smoothing multiplier for the RSI calculation.
/// * `rsi_line` - A mutable slice for storing the RSI output values.
/// * `state` - A mutable reference to the current RSI `State`.
fn cycle_rsi(real: &[f64], multipliers: (f64, f64), rsi_line: &mut [f64], state: &mut State) {
    for i in 0..real.len() {
        unsafe { *rsi_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i), multipliers) };
    }
}

