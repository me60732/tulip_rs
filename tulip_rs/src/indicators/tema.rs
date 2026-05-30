use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::dema::{
    calc as calc_dema, output_length as dema_output_length, State as DemaState,
};
pub use crate::indicators::ema::multiplier;
use crate::indicators::ema::{calc as calc_ema, output_length as ema_output_length};
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
pub use crate::indicators::simd_indicators::tema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::tema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::tema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::tema_simd::indicator_by_options as indicator;
}

/// Returns information about the Triple Exponential Moving Average (TEMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TEMA indicator.
pub const INFO: Info = Info {
    name: "tema",
    indicator_type: IndicatorType::Trend,
    full_name: "Triple Exponential Moving Average",
    inputs: &["real"],
    options: &["period"],
    outputs: &["tema"],
    optional_outputs: &["dema", "ema"],
    display_groups: &[
        DisplayGroup {
            id: "tema",
            label: "EMAs",
            display_type: DisplayType::Overlay,
            outputs: &["tema", "dema", "ema"],
        },
    ],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub dema_state: DemaState,
    pub ema3: f64,
}
impl State {
    pub fn new(ema1: f64, ema2: f64, ema3: f64) -> Self {
        Self {
            dema_state: DemaState::new(ema1, ema2),
            ema3,
        }
    }
    pub fn init_state(
        real: &[f64],
        period: usize,
        tema_capacity: usize,
        out_vecs: (&mut [f64], &mut [f64]),
    ) -> Self {
        //let mut remaining = real.len();
        let multiplier = multiplier(period);
        let (dema_line, ema_line) = out_vecs;
        let dema_capacity = dema_output_length(real.len(), &[period as f64]);
        let mut state = Self {
            dema_state: DemaState::init_state(real, dema_capacity, period, ema_line),
            ema3: 0.0,
        };
        let mut i = real.len() - dema_capacity;
        let remaining = real.len() - tema_capacity;
        while i < remaining {
            let value = &real[i];
            let (_, dema, ema) = calc(&mut state, value, multiplier);
            if i == real.len() - dema_capacity {
                state.ema3 = state.dema_state.ema2;
            }
            crate::init_store_optional_outputs!(i, real.len(),
                dema_line => dema,
                ema_line => ema
            );
            i += 1;
            //remaining -= 1;
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

        let (mut tema_line, mut dema_line, mut ema_line);
        {
            let capacity = inputs[0].len();
            tema_line = crate::uninit_vec!(f64, capacity);
            (dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                dema_line: capacity,
                ema_line: capacity
            );
        }
        cycle_tema(
            inputs[0],
            self.multipliers,
            &mut self.state,
            &mut tema_line,
            (&mut dema_line, &mut ema_line),
        );
        Ok(vec![tema_line, dema_line, ema_line])
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
/// Returns the minimum amount of data required for the TEMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the TEMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize * 3 - 2
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TEMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Triple Exponential Moving Average (TEMA) indicator over the full input dataset.
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
/// * `optional_outputs` - Optional slice controlling extra output series;
///   `optional_outputs[0] = true` enables `dema`, `optional_outputs[1] = true` enables `ema`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `tema`,
/// `outputs[1]` is `dema` (empty unless requested),
/// `outputs[2]` is `ema` (empty unless requested), and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;
    let (mut tema_line, mut dema_line, mut ema_line, mut state, multipliers, real);
    {
        let len = inputs[0].len();
        let capacity = output_length(len, options);
        let ema_capacity = ema_output_length(len, options);
        let dema_capacity = dema_output_length(len, options);

        tema_line = crate::uninit_vec!(f64, capacity);

        (dema_line, ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            dema_line: dema_capacity,
            ema_line: ema_capacity
        );
        let period = options[0] as usize;
        state = State::init_state(inputs[0], period, capacity, (&mut dema_line, &mut ema_line));
        let start = len - capacity;
        real = &inputs[0][start..];
        multipliers = multiplier(period);
    }
    let optional_outputs = {
        let offsets = crate::slice_outputs_start!(tema_line.len(), dema_line, ema_line);
        (&mut dema_line[offsets.0..], &mut ema_line[offsets.1..])
    };

    // Perform the main TEMA calculation
    cycle_tema(
        real,
        multipliers,
        &mut state,
        &mut tema_line,
        optional_outputs,
    );

    Ok((
        vec![tema_line, dema_line, ema_line],
        IndicatorState { multipliers, state },
    ))
}

/// Performs the main calculation loop for the TEMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `multipliers` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
/// * `state` - A mutable reference to the current indicator state.
/// * `tema_line` - A mutable slice for storing the TEMA output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(dema_line, ema_line)`.
fn cycle_tema(
    real: &[f64],
    multipliers: (f64, f64),
    state: &mut State,
    tema_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (dema_line, ema_line) = out_vecs;
    let (has_optional, want_dema, want_ema) = crate::calc_want_flags!(dema_line, ema_line);

    for i in 0..real.len() {
        let value = unsafe { real.get_unchecked(i) };
        let (tema, dema, ema) = calc(state, value, multipliers);
        unsafe { *tema_line.get_unchecked_mut(i) = tema };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_dema, dema_line => dema,
                want_ema, ema_line => ema
            );
        }
    }
}

/// Calculates a single TEMA value from the current state.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current indicator state.
/// * `value` - The current input value.
/// * `multiplier` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
///
/// # Returns
///
/// A tuple `(tema, dema, ema)` containing the current TEMA, DEMA, and EMA values.
#[inline(always)]
pub fn calc(state: &mut State, value: &f64, multiplier: (f64, f64)) -> (f64, f64, f64) {
    let dema_state = &mut state.dema_state;
    let (dema, ema) = calc_dema(dema_state, value, multiplier);
    state.ema3 = calc_ema(&dema_state.ema2, state.ema3, multiplier);

    (
        //3.0 * dema_state.ema1 - 3.0 * dema_state.ema2 + state.ema3,
        dema_state
            .ema1
            .mul_add(3.0, dema_state.ema2.mul_add(-3.0, state.ema3)),
        dema,
        ema,
    )
}
