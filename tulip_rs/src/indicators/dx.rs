use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::di::calc_diup_didown;
pub use crate::indicators::di::State;
use crate::indicators::tr::output_length as tr_output_length;
pub use crate::indicators::wilders::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dx_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    inv_multiplier: f64,
}
impl IndicatorState {
    pub fn new(state: State, inv_multiplier: f64) -> Self {
        Self {
            state,
            inv_multiplier,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        //let mut dx_line = vec![0.0; capacity];
        let (mut dx_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            dx_line = crate::uninit_vec!(f64, capacity);
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        let [high, low, close] = inputs;
        cycle(
            high,
            low,
            close,
            &mut self.state,
            self.inv_multiplier,
            (&mut dx_line, &mut atr_line, &mut tr_line),
        );
        Ok(vec![dx_line, atr_line, tr_line])
    }
}
/// Returns information about the Directional Movement Index (DX) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DX indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "dx",
        full_name: "Directional Movement Index",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["dx"],
        optional_outputs: &["atr", "tr"],
    }
}
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// Because DX uses Wilder's smoothing the seed value's influence must
/// decay below the requested precision, so this value grows with
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
        &[multiplier(options[0] as usize).1],
        IndicatorInfoOrInteger::Integer(1),
        min_data,
    )
}
/// Returns the minimum amount of data required for the DX indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DX calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1 // period
}
/// Returns the number of output values produced by the DX indicator given input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the DX calculation.
///
/// # Returns
///
/// The number of output values.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Directional Movement Index (DX) indicator for an entire dataset.
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
/// # Outputs
///
/// * `outputs[0]` — `dx` line
/// * `outputs[1]` — `atr` (optional, if requested)
/// * `outputs[2]` — `tr` (optional, if requested)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice selecting which extra outputs to compute:
///   index `0` = `atr`, index `1` = `tr`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `dx` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let (_, inv_multiplier) = multiplier(period);

    validate_inputs(inputs, min_data(options))?;

    let capacity = output_length(inputs[0].len(), options);
    let tr_capacity = tr_output_length(inputs[0].len(), options);
    let (mut dx_line, mut atr_line, mut tr_line);
    {
        dx_line = crate::uninit_vec!(f64, capacity);
        (atr_line, tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            atr_line: capacity,
            tr_line: tr_capacity
        );
    }

    let mut state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line);
    let tr = {
        let offset = crate::slice_outputs_start!(dx_line.len(), tr_line);
        &mut tr_line[offset..]
    };
    let (high, low, close) = (
        &inputs[0][period..],
        &inputs[1][period..],
        &inputs[2][period..],
    );
    cycle(
        high,
        low,
        close,
        &mut state,
        inv_multiplier,
        (&mut dx_line, &mut atr_line, tr),
    );

    Ok((
        vec![dx_line, atr_line, tr_line],
        IndicatorState {
            state,
            inv_multiplier,
        },
    ))
}

/// Performs the main calculation loop for the DX indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - A mutable reference to the indicator state.
/// * `inv_multiplier` - The inverse smoothing multiplier used for ATR calculation.
/// * `out_vecs` - A tuple of mutable output slices: `(dx_line, atr_line, tr_line)`.
//#[inline(always)]
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State,
    inv_multiplier: f64,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (dx_line, atr_line, tr_line) = out_vecs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (dx, atr, tr) = calc(state, h, l, c);
        unsafe {
            *dx_line.get_unchecked_mut(i) = dx;
        }
        if has_optional {
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, inv_multiplier)
            );
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr
            );
        }
    }
}
/// Calculates the current DX, ATR, and TR values for one bar.
///
/// # Arguments
///
/// * `state` - A mutable reference to the indicator state.
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
///
/// # Returns
///
/// A tuple `(dx, atr, tr)` containing the current DX value, ATR, and True Range.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64, f64) {
    let (_, _, atr, tr) = calc_diup_didown(state, high, low, close);

    let dx = calc_dx(state);

    (dx, atr, tr)
}
#[inline(always)]
pub(crate) fn calc_dx(state: &mut State) -> f64 {
    let di_up = state.di_state.dmup / state.atr_state.atr;
    let di_down = state.di_state.dmdown / state.atr_state.atr;

    let dm_diff = (di_up - di_down).abs();
    let dm_sum = di_up + di_down;
    (dm_diff * 100.0 / dm_sum).max(0.0)
}
