use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::adx::multiplier;
use crate::indicators::adx::{
    calc as calc_adx, output_length as adx_output_length, State as AdxState,
};

use crate::indicators::dx::output_length as dx_output_length;
use crate::indicators::tr::output_length as tr_output_length;
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::adxr_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::adxr_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::adxr_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::adxr_simd::indicator_by_options as indicator;
}

/// Returns information about the Average Directional Movement Rating (ADXR) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the ADXR indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "adxr",
        full_name: "Average Directional Movement Rating",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["adxr"],
        optional_outputs: &["adx", "dx", "atr", "tr"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub adx_state: AdxState,
    pub buffer: Buffer<f64>,
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
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let capacity = inputs[0].len();
        //let mut adxr_line = vec![0.0; capacity]; //Vec::with_capacity(capacity);
        let mut adxr_line: Vec<f64> = Vec::with_capacity(capacity);
        unsafe {
            adxr_line.set_len(capacity);
        }
        let (mut adx_line, mut dx_line, mut atr_line, mut tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false, false],
            adx_line: capacity,
            dx_line: capacity,
            atr_line: capacity,
            tr_line: capacity
        );

        cycle_adxr(
            &high,
            &low,
            &close,
            &mut self.state,
            self.inv_multiplier,
            &mut adxr_line,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );
        Ok(vec![adxr_line, adx_line, dx_line, atr_line, tr_line])
    }
}
impl State {
    pub fn new(adx_state: AdxState, buffer: Buffer) -> Self {
        Self { adx_state, buffer }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> State {
        let (adx_line, dx_line, atr_line, tr_line) = out_vecs;
        let mut adx_state =
            AdxState::init_state(high, low, close, period, (dx_line, atr_line, tr_line));
        let mut prev_adx = Buffer::new(period - 1);
        prev_adx.push(adx_state.adx);

        let mut i = period * 2 - 1;
        let (_, inv_multiplier) = multiplier(period);
        while !prev_adx.is_full() {
            let (adx, dx, atr, tr) = calc_adx(&mut adx_state, high[i], low[i], close[i]);
            prev_adx.push(adx);
            crate::init_store_optional_outputs!(i, high.len(),
                adx_line => adx,
                dx_line => dx,
                atr_line => atr * inv_multiplier,
                tr_line => tr
            );
            i += 1;
        }

        State::new(adx_state, prev_adx)
    }
}
/// Returns the minimum amount of data required based on the given options.
///
/// # Arguments
///
/// * `options` - A slice containing the period for the ADXR calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    (options[0] as usize - 1) * 3 + 1 // period
}
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with Wilder's smoothing
/// multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (period).
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
        IndicatorInfoOrInteger::Integer(1),
        min_data,
    )
}
/// Calculates the output length for the ADXR indicator based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the period for the ADXR calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Average Directional Movement Index Rating (ADXR) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (must be >= 2)
///
/// # Arguments
///
/// * `inputs` - Array of 3 input price slices (see Inputs above).
/// * `options` - Array of 1 indicator option (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false, false, false])` to enable individual
///   optional outputs (`adx`, `dx`, `atr`, `tr`); `None` disables all.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `adxr` line,
/// `outputs[1]` is the optional `adx` line, `outputs[2]` is the optional `dx` line,
/// `outputs[3]` is the optional `atr` line, and `outputs[4]` is the optional `tr` line
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
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    /*let mut adxr_line: Vec<f64> = Vec::with_capacity(adxr_capacity);
    unsafe { adxr_line.set_len(adxr_capacity); }*/
    //let mut adxr_line = vec![0.0; adxr_capacity]; // Vec::with_capacity(adxr_capacity);
    let (mut adxr_line, (mut adx_line, mut dx_line, mut atr_line, mut tr_line)) = {
        let len = inputs[0].len();
        let adxr_capacity = output_length(len, options);
        let adx_capacity = adx_output_length(len, options);
        let dx_capacity = dx_output_length(len, options);
        let tr_capacity = tr_output_length(len, options);

        (
            crate::uninit_vec!(f64, adxr_capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                adx_line: adx_capacity,
                dx_line: dx_capacity,
                atr_line: dx_capacity,
                tr_line: tr_capacity
            ),
        )
    };
    let inv_multiplier = multiplier(period).1;
    let mut state = State::init_state(
        inputs[0], // high
        inputs[1], //low
        inputs[2], //close
        period,
        (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
    );
    let (high, low, close) = {
        let from = inputs[0].len() - adxr_line.len();
        (&inputs[0][from..], &inputs[1][from..], &inputs[2][from..])
    };
    let outputs = {
        let offsets =
            crate::slice_outputs_start!(adxr_line.len(), adx_line, dx_line, atr_line, tr_line);
        (
            &mut adx_line[offsets.0..],
            &mut dx_line[offsets.1..],
            &mut atr_line[offsets.2..],
            &mut tr_line[offsets.3..],
        )
    };

    cycle_adxr(
        high,
        low,
        close,
        &mut state,
        inv_multiplier,
        &mut adxr_line,
        outputs,
    );

    Ok((
        vec![adxr_line, adx_line, dx_line, atr_line, tr_line],
        IndicatorState {
            state,
            inv_multiplier,
        },
    ))
}

/// Performs the main calculation loop for the ADXR indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - A mutable reference to the current `State` (ADX state and the rolling ADX buffer).
/// * `inv_multiplier` - The inverse ATR multiplier used to scale ATR values.
/// * `adxr_line` - A mutable slice for storing the resulting ADXR line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: ADX, DX, ATR, and TR lines.
#[inline(always)]
fn cycle_adxr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State,
    inv_multiplier: f64,
    adxr_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (adx_line, dx_line, atr_line, tr_line) = out_vecs;
    let (has_optional, want_adx, want_dx, want_atr, want_tr) =
        crate::calc_want_flags!(adx_line, dx_line, atr_line, tr_line);

    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (adxr, adx, dx, atr, tr) = unsafe { calc_unchecked(state, h, l, c) };

        unsafe {
            *adxr_line.get_unchecked_mut(i) = adxr;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_adx, adx_line => adx,
                want_dx, dx_line => dx,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, inv_multiplier)
            );
        }
    }
}

#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64, f64, f64, f64) {
    let (adx, dx, atr, tr) = calc_adx(&mut state.adx_state, high, low, close);

    let prev_adx = state.buffer.push_with_info(adx);
    let mut adxr = 0.0;
    if let Some(pa) = prev_adx {
        adxr = 0.5 * (adx + pa);
    }

    (adxr, adx, dx, atr, tr)
}
#[inline(always)]
pub unsafe fn calc_unchecked(
    state: &mut State,
    high: f64,
    low: f64,
    close: f64,
) -> (f64, f64, f64, f64, f64) {
    let (adx, dx, atr, tr) = calc_adx(&mut state.adx_state, high, low, close);
    let adxr = 0.5 * (adx + state.buffer.push_with_info_unchecked(adx));

    (adxr, adx, dx, atr, tr)
}
/*#[inline(always)]
fn multiplier(period: usize) -> f64 {
    adx_multiplier(period)
}*/
