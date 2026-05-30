use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::atr::calc as calc_atr;
pub use crate::indicators::atr::multiplier;
pub use crate::indicators::atr::State;
use crate::indicators::tr::output_length as tr_output_length;
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::natr_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::natr_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::natr_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::natr_simd::indicator_by_options as indicator;
}

/// Returns information about the Normalized Average True Range (NATR) indicator.
pub const INFO: Info = Info {
    name: "natr",
    full_name: "Normalized Average True Range",
    indicator_type: IndicatorType::Volatility,
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["natr"],
    optional_outputs: &["atr", "tr"],
    display_groups: &[
        DisplayGroup {
            id: "natr",
            label: "NATR",
            display_type: DisplayType::Indicator,
            outputs: &["natr"],
        },
        DisplayGroup {
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        },
    ],
};
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
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the NATR indicator.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Returns the output length for the NATR indicator.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
}
impl IndicatorState {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut natr_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            natr_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        cycle_natr(
            (inputs[0], inputs[1], inputs[2]),
            &mut natr_line,
            (&mut atr_line, &mut tr_line),
            &mut self.state,
        );

        Ok(vec![natr_line, atr_line, tr_line])
    }
}

/// Calculates the Normalized Average True Range (NATR) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice of booleans enabling extra outputs:
///   `[0]` → `atr`, `[1]` → `tr`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `natr` line,
/// `outputs[1]` is the `atr` line (empty if not requested), and
/// `outputs[2]` is the `tr` line (empty if not requested). `state` can be
/// passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let (mut natr_line, mut atr_line, mut tr_line);
    {
        let capacity = output_length(inputs[0].len(), options);
        natr_line = crate::uninit_vec!(f64, capacity);

        (atr_line, tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            atr_line: capacity,
            tr_line: tr_output_length(inputs[0].len(), options)
        );
    }
    let mut state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line, false);
    let offset = crate::slice_outputs_start!(natr_line.len(), tr_line);

    cycle_natr(
        (
            &inputs[0][period..],
            &inputs[1][period..],
            &inputs[2][period..],
        ),
        &mut natr_line,
        (&mut atr_line, &mut tr_line[offset..]),
        &mut state,
    );

    Ok((
        vec![natr_line, atr_line, tr_line],
        IndicatorState { state: state },
    ))
}

/// Iterates over the input data and applies the calc function.
//#[inline(always)]
fn cycle_natr(
    inputs: (&[f64], &[f64], &[f64]),
    natr_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
    state: &mut State,
) {
    let (high, low, close) = inputs;
    let (atr_line, tr_line) = out_vecs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (natr, atr, tr) = calc(state, h, l, c);
        unsafe { *natr_line.get_unchecked_mut(i) = natr };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr
            );
        }
    }
}

/// Performs the core calculation for the Normalized Average True Range (NATR) indicator.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64, f64) {
    let (atr, tr) = calc_atr(state, high, low, close);
    ((atr / close) * 100.0, atr, tr)
}
