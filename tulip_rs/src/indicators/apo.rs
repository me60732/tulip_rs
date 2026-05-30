use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::{
    calc as ema_calc, multiplier as ema_multiplier, output_length as ema_output_length,
};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::apo_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::apo_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::apo_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::apo_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64), (f64, f64)),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, multipliers: ((f64, f64), (f64, f64))) -> Self {
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

        let capacity = inputs[0].len();
        let mut apo_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: capacity,
            long_ema_line: capacity
        );

        cycle_apo(
            inputs[0],
            &mut self.state,
            self.multipliers,
            &mut apo_line,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![apo_line, short_ema_line, long_ema_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_ema: f64,
    pub long_ema: f64,
}
impl State {
    pub fn new(short_ema: f64, long_ema: f64) -> Self {
        State {
            short_ema,
            long_ema,
        }
    }
    pub fn init_state(
        real: &[f64],
        short_period: usize,
        long_period: usize,
        short_ema_line: &mut [f64],
    ) -> State {
        let (short_multiplier, long_multiplier) = multiplier(short_period, long_period);
        let (mut short_ema, mut long_ema) = (real[0], real[0]);

        for (i, value) in real.iter().enumerate().take(long_period - 1).skip(1) {
            short_ema = ema_calc(value, short_ema, short_multiplier);
            long_ema = ema_calc(value, long_ema, long_multiplier);
            crate::init_store_optional_outputs!(i, real.len(),
                short_ema_line => short_ema
            );
        }
        State::new(short_ema, long_ema)
    }
}
/// Returns information about the Absolute Price Oscillator (APO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the APO indicator.
pub const INFO: Info = Info {
    name: "apo",
    full_name: "Absolute Price Oscillator",
    indicator_type: IndicatorType::Momentum,
    inputs: &["close"],
    options: &["short_period", "long_period"],
    outputs: &["apo"],
    optional_outputs: &["short_ema", "long_ema"],
    display_groups: &[
        DisplayGroup {
            id: "apo",
            label: "APO",
            display_type: DisplayType::Indicator,
            outputs: &["apo"],
        },
        DisplayGroup {
            id: "short_ema_long_ema",
            label: "EMA",
            display_type: DisplayType::Overlay,
            outputs: &["short_ema", "long_ema"],
        },
    ],
};
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the long-period EMA
/// smoothing multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (short period, long period).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let (_short_multiplier, long_multiplier) = multiplier(options[0] as usize, options[1] as usize);
    min_process(
        options,
        Some((decimals, 0)),
        &[long_multiplier.0],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the APO indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the APO calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize
}

/// Calculates the output length for the APO indicator based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the short and long periods for the APO calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Absolute Price Oscillator (APO) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close prices
///
/// # Options
///
/// * `options[0]` — short period (must be >= 1)
/// * `options[1]` — long period (must be > short period)
///
/// # Arguments
///
/// * `inputs` - Array of 1 input price slice (see Inputs above).
/// * `options` - Array of 2 indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable individual
///   optional outputs (`short_ema`, `long_ema`); `None` disables all.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `apo` line,
/// `outputs[1]` is the optional `short_ema` line, and `outputs[2]` is the optional `long_ema` line
/// (each empty if not requested).
/// `state` can be passed to `IndicatorState::batch_indicator` to continue streaming.
///
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    let short_period = options[0] as usize;
    let long_period = options[1] as usize;

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];

    let capacity = output_length(real.len(), options);
    let short_ema_capacity = ema_output_length(real.len(), &[short_period as f64]);

    let mut apo_line = crate::uninit_vec!(f64, capacity);

    let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs_eff!(
        optional_outputs, &[false, false],
        short_ema_line: short_ema_capacity,
        long_ema_line: capacity
    );

    let multipliers = multiplier(short_period, long_period);
    let mut state = State::init_state(real, short_period, long_period, &mut short_ema_line);

    let optional_outputs = {
        let short_start = crate::slice_outputs_start!(capacity, short_ema_line);
        (
            &mut short_ema_line[short_start..],
            long_ema_line.as_mut_slice(),
        )
    };

    cycle_apo(
        &real[real.len() - apo_line.len()..],
        &mut state,
        multipliers,
        &mut apo_line,
        optional_outputs,
    );

    Ok((
        vec![apo_line, short_ema_line, long_ema_line],
        IndicatorState { state, multipliers },
    ))
}

/// Performs the main calculation loop for the APO indicator.
///
/// # Arguments
///
/// * `real` - A slice of close prices to process.
/// * `state` - A mutable reference to the current `State` (short EMA, long EMA).
/// * `multipliers` - The precomputed EMA multipliers for the short and long periods.
/// * `apo_line` - A mutable slice for storing the resulting APO line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: short EMA and long EMA lines.
fn cycle_apo(
    real: &[f64],
    state: &mut State,
    multipliers: ((f64, f64), (f64, f64)),
    apo_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (short_ema_line, long_ema_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        unsafe { *apo_line.get_unchecked_mut(i) = calc(state, real.get_unchecked(i), multipliers) };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => state.short_ema,
                want_long, long_ema_line => state.long_ema
            );
        }
    }
}

#[inline(always)]
pub fn calc(state: &mut State, real: &f64, multipliers: ((f64, f64), (f64, f64))) -> f64 {
    let (short_multiplier, long_multiplier) = multipliers;
    state.short_ema = ema_calc(real, state.short_ema, short_multiplier);
    state.long_ema = ema_calc(real, state.long_ema, long_multiplier);

    state.short_ema - state.long_ema
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}
