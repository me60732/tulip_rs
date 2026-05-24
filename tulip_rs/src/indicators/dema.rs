use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::ema::multiplier;
use crate::indicators::ema::{calc as calc_ema, output_length as ema_output_length};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dema_simd::indicator_by_options as indicator;
}

/// Returns information about the Double Exponential Moving Average (DEMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DEMA indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "dema",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Double Exponential Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["dema"],
        optional_outputs: &["ema"],
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub ema1: f64,
    pub ema2: f64,
}
impl State {
    pub fn new(ema1: f64, ema2: f64) -> Self {
        Self { ema1, ema2 }
    }
    pub fn init_state(real: &[f64], capacity: usize, period: usize, ema_line: &mut [f64]) -> Self {
        let mut remaining = real.len();
        let mut i = 1;
        let mut ema1 = real[0];
        let mut state = Self::new(0.0, 0.0);

        let multiplier = multiplier(period);
        while capacity < remaining - 1 {
            if i < period {
                ema1 = calc_ema(&real[i], ema1, multiplier);
                state.ema1 = ema1;
                state.ema2 = ema1;
            } else if i >= period {
                _ = calc(&mut state, &real[i], multiplier);
            }

            crate::init_store_optional_outputs!(i, real.len(),
                ema_line => state.ema1
            );
            i += 1;
            remaining -= 1;
        }

        state
    }
}

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
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut dema_line, mut ema_line) = {
            let capacity = inputs[0].len();
            //let mut dema_line = vec![0.0; capacity];
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                ),
            )
        };
        cycle_dema(
            inputs[0],
            self.multipliers,
            &mut self.state,
            &mut dema_line,
            &mut ema_line,
        );

        Ok(vec![dema_line, ema_line])
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
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the DEMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DEMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize * 2 - 1
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the DEMA calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    //println!("Len: {:?}, Options: {:?}", data_len, options);
    data_len - min_data(options) + 1
}

/// Calculates the Double Exponential Moving Average (DEMA) over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real values (typically close prices)
///
/// # Options
///
/// * `options[0]` — period (EMA window length)
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to enable the optional `ema`
///   output; `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `dema` and `outputs[1]` is `ema`
/// (empty unless requested). `state` can be passed to `IndicatorState::batch_indicator`
/// for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);
    validate_inputs(inputs, min_data(options))?;

    let (mut dema_line, mut ema_line, mut state);
    {
        let capacity = output_length(inputs[0].len(), options);
        let ema_capacity = ema_output_length(inputs[0].len(), options);

        dema_line = crate::uninit_vec!(f64, capacity);

        // Initialize any optional outputs
        ema_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            ema_line: ema_capacity
        );

        state = State::init_state(inputs[0], capacity, period, &mut ema_line);
    }
    let ema = {
        let offset = crate::slice_outputs_start!(dema_line.len(), ema_line);
        &mut ema_line[offset..]
    };

    cycle_dema(
        &inputs[0][period * 2 - 2..],
        multipliers,
        &mut state,
        &mut dema_line,
        ema,
    );

    Ok((
        vec![dema_line, ema_line],
        IndicatorState { state, multipliers },
    ))
}

/// Performs the main calculation loop for the DEMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `multipliers` - A tuple of EMA multipliers derived from the period.
/// * `state` - Mutable reference to the DEMA state holding `ema1` and `ema2`.
/// * `dema_line` - Mutable slice to write the DEMA output values into.
/// * `ema_line` - Mutable slice to write the EMA output values into (optional output).
fn cycle_dema(
    real: &[f64],
    multipliers: (f64, f64),
    state: &mut State,
    dema_line: &mut [f64],
    ema_line: &mut [f64],
) {
    let (_, want_ema) = crate::calc_want_flags!(ema_line);

    for i in 0..real.len() {
        let value = unsafe { real.get_unchecked(i) };

        let (dema, ema) = calc(state, value, multipliers);

        unsafe { *dema_line.get_unchecked_mut(i) = dema };

        crate::store_optional_outputs!(i,
            want_ema, ema_line => ema
        );
    }
}

/// Calculates the Double Exponential Moving Average (DEMA) for the current data point.
///
/// # Arguments
///
/// * `state` - Mutable reference to the DEMA state holding `ema1` and `ema2`.
/// * `value` - The current input value.
/// * `multiplier` - A tuple of EMA multipliers derived from the period.
///
/// # Returns
///
/// A tuple `(dema, ema1)` representing the DEMA value and the updated first EMA.
#[inline(always)]
pub fn calc(state: &mut State, value: &f64, multiplier: (f64, f64)) -> (f64, f64) {
    state.ema1 = calc_ema(value, state.ema1, multiplier);
    state.ema2 = calc_ema(&state.ema1, state.ema2, multiplier);
    //(2.0 * state.ema1 - state.ema2, state.ema1)
    (state.ema1.mul_add(2.0, -state.ema2), state.ema1)
}
