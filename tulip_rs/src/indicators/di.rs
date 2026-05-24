use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::atr::{partial_calc as partial_calc_atr, State as AtrState};
use crate::indicators::dm::{calc as calc_dm, State as DMState};
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
pub use crate::indicators::simd_indicators::di_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::di_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::di_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::di_simd::indicator_by_options as indicator;
}

/// Returns information about the Directional Indicator (DI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "di",
        full_name: "Directional Indicator",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["plus_di", "minus_di"],
        optional_outputs: &["atr", "tr"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub di_state: DMState,
    pub atr_state: AtrState,
}
impl State {
    pub fn new(dm_state: (f64, f64, f64, f64), atr_state: (f64, f64), multiplier: f64) -> Self {
        Self {
            atr_state: AtrState::new(atr_state.0, atr_state.1, multiplier),
            di_state: DMState::new(dm_state.0, dm_state.1, dm_state.2, dm_state.3, multiplier),
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State {
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, true);
        let di_state = DMState::init_state(high, low, period);

        State {
            atr_state,
            di_state,
        }
    }
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

        let (mut plus_di_line, mut minus_di_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            plus_di_line = crate::uninit_vec!(f64, capacity);
            minus_di_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        let [high, low, close] = inputs;
        cycle_calc(
            high,
            low,
            close,
            &mut self.state,
            self.inv_multiplier,
            (&mut plus_di_line, &mut minus_di_line),
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![plus_di_line, minus_di_line, atr_line, tr_line])
    }
}
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the Wilder's smoothing
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
/// Returns the minimum amount of data required for the DI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the DI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1 // period
}
/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the DI calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Directional Indicator (DI) over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (Wilder smoothing window for DM+ / DM- / ATR)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable `atr`,
///   `Some(&[false, true])` to enable `tr`, or `Some(&[true, true])` for both;
///   `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `plus_di`, `outputs[1]` is `minus_di`,
/// `outputs[2]` is `atr`, and `outputs[3]` is `tr` (empty unless requested).
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
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let (mut plus_di_line, mut minus_di_line, mut atr_line, mut tr_line);
    {
        let capacity = output_length(high.len(), options);
        let tr_capacity = tr_output_length(high.len(), options);

        plus_di_line = crate::uninit_vec!(f64, capacity);
        minus_di_line = crate::uninit_vec!(f64, capacity);

        (atr_line, tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            atr_line: capacity,
            tr_line: tr_capacity
        );
    }
    let mut state = State::init_state(high, low, close, period, &mut tr_line);
    let tr = {
        let offsets = crate::slice_outputs_start!(plus_di_line.len(), tr_line);
        &mut tr_line[offsets..]
    };
    let (high, low, close) = { (&high[period..], &low[period..], &close[period..]) };
    cycle_calc(
        high,
        low,
        close,
        &mut state,
        inv_multiplier,
        (&mut plus_di_line, &mut minus_di_line),
        (&mut atr_line, tr),
    );

    Ok((
        vec![plus_di_line, minus_di_line, atr_line, tr_line],
        IndicatorState {
            state: state,
            inv_multiplier,
        },
    ))
}

/// Performs the main calculation loop for the DI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - Mutable reference to the DI state (DM and ATR sub-states).
/// * `inv_multiplier` - The inverse Wilder's multiplier used to scale ATR output.
/// * `outputs` - A tuple of `(plus_di_line, minus_di_line)` output slices.
/// * `out_vecs` - A tuple of `(atr_line, tr_line)` for optional outputs.
fn cycle_calc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State,
    inv_multiplier: f64,
    outputs: (&mut [f64], &mut [f64]),
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (plus_di_line, minus_di_line) = outputs;
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

        let (pdi, mdi, atr, tr) = calc(state, h, l, c);

        unsafe {
            *plus_di_line.get_unchecked_mut(i) = pdi;
            *minus_di_line.get_unchecked_mut(i) = mdi;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, inv_multiplier)
            );
        }
    }
}

/// Calculates the current Directional Indicator (DI) values.
///
/// # Arguments
///
/// * `state` - Mutable reference to the DI state (DM and ATR sub-states).
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
///
/// # Returns
///
/// A tuple `(plus_di, minus_di, atr, tr)` representing the current DI values,
/// the smoothed ATR, and the raw true range.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64, f64, f64) {
    let (dmup, dmdown, atr, tr) = calc_diup_didown(state, high, low, close);
    //let pdi = 100.0 * dmup / atr;
    //let mdi = 100.0 * dmdown / atr;
    // Fix the division by zero/NaN issue
    /* if atr <= 0.0 || atr.is_nan() {
        return (0.0, 0.0, atr, tr);  // Return safe values when ATR is invalid
    }*/
    let atr_inv = 100.0 / atr;
    let mut pdi = dmup * atr_inv; // multiplication
    let mut mdi = dmdown * atr_inv;
    pdi = if pdi.is_nan() { 0.0 } else { pdi };
    mdi = if mdi.is_nan() { 0.0 } else { mdi };
    (pdi, mdi, atr, tr)
}

#[inline(always)]
pub fn calc_diup_didown(
    state: &mut State,
    high: f64,
    low: f64,
    close: f64,
) -> (f64, f64, f64, f64) {
    let (atr, tr) = partial_calc_atr(&mut state.atr_state, high, low, close);
    let (dmup, dmdown) = calc_dm(&mut state.di_state, high, low);
    (dmup, dmdown, atr, tr)
}
