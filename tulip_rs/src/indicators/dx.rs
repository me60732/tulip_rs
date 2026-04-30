use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::di::calc_diup_didown;
pub use crate::indicators::di::State;
use crate::indicators::tr::output_length as tr_output_length;
pub use crate::indicators::wilders::multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
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
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the DX calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Calculates the Directional Movement Index (DX) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `options` - A slice containing the period for the DX calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the DX line and any additional requested outputs.

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
/// * `period` - The period for the DX calculation.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
/// * `dx_line` - A mutable reference to a vector for storing the DX line.
/// * `output_vectors` - A mutable reference to an array of optional vectors for storing additional outputs.
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
/// Calculates the current value of the Directional Movement Index (DX).
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `prev_high` - The previous high price.
/// * `prev_low` - The previous low price.
/// * `prev_close` - The previous close price.
/// * `prev_plus_di` - The previous plus DI value.
/// * `prev_minus_di` - The previous minus DI value.
/// * `prev_atr` - The previous ATR value.
/// * `period` - The period for the DX calculation.
///
/// # Returns
///
/// A tuple containing the current plus DI value, the current minus DI value, and the updated ATR value.
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
