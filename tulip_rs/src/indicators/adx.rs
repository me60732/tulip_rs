use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::dx::{
    calc as current_dx, calc_dx, output_length as dx_output_length, State as DxState,
};
use crate::indicators::tr::output_length as tr_output_length;
use crate::indicators::wilders::calc as calc_wilders;
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
pub use crate::indicators::simd_indicators::adx_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::adx_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::adx_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset in parallel with `N` option sets.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::adx_simd::indicator_by_options as indicator;
}

/// Returns information about the Average Directional Index (ADX) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the ADX indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "adx",
        full_name: "Average Directional Index",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["adx"],
        optional_outputs: &["dx", "atr", "tr"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub dx_state: DxState,
    pub adx: f64,
    multiplier: f64,
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

        let (mut adx_line, mut dx_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();

            adx_line = crate::uninit_vec!(f64, capacity);
            (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                dx_line: capacity,
                atr_line: capacity,
                tr_line: capacity
            );
        }

        cycle_adx(
            (inputs[0], inputs[1], inputs[2]),
            self.inv_multiplier,
            &mut self.state,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );

        Ok(vec![adx_line, dx_line, atr_line, tr_line])
    }
}
impl State {
    pub fn new(
        adx: f64,
        dm_state: (f64, f64, f64, f64),
        atr_state: (f64, f64),
        multiplier: f64,
    ) -> Self {
        Self {
            adx,
            dx_state: DxState::new(dm_state, atr_state, multiplier),
            multiplier,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State {
        let (dx_line, atr_line, tr_line) = out_vecs;
        let (multiplier, inv_multiplier) = multiplier(period);
        let mut dx_state = DxState::init_state(high, low, close, period, tr_line);
        let mut adx = calc_dx(&mut dx_state);
        for (i, ((&h, &l), &c)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .take(period * 2 - 1)
            .skip(period)
        {
            let (dx, atr, tr) = current_dx(&mut dx_state, h, l, c);
            adx += dx;
            crate::init_store_optional_outputs!(i, high.len(),
                dx_line => dx,
                atr_line => atr * inv_multiplier,
                tr_line => tr
            );
        }
        adx /= period as f64;
        State {
            dx_state,
            adx,
            multiplier,
        }
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
/// Returns the minimum amount of data required for the ADX indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the ADX calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize * 2 // period
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the ADX calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Average Directional Index (ADX) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `options` - A slice containing the period for the ADX calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the ADX line and any additional requested outputs.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;

    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let (mut adx_line, mut dx_line, mut atr_line, mut tr_line);
    {
        let dx_capacity = dx_output_length(inputs[0].len(), options);
        let adx_capacity = output_length(inputs[0].len(), options);
        let tr_capacity = tr_output_length(inputs[0].len(), &[]);
        adx_line = crate::uninit_vec!(f64, adx_capacity);

        (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            dx_line: dx_capacity,
            atr_line: dx_capacity,
            tr_line: tr_capacity
        );
    }
    let inv_multiplier = multiplier(period).1;
    let mut state = State::init_state(
        high,
        low,
        close,
        period,
        (&mut dx_line, &mut atr_line, &mut tr_line),
    );
    let outputs = {
        let offsets = crate::slice_outputs_start!(adx_line.len(), dx_line, atr_line, tr_line);
        (
            adx_line.as_mut_slice(),
            &mut dx_line[offsets.0..],
            &mut atr_line[offsets.1..],
            &mut tr_line[offsets.2..],
        )
    };
    let inputs = {
        let from = period * 2 - 1;
        (&high[from..], &low[from..], &close[from..])
    };
    cycle_adx(inputs, inv_multiplier, &mut state, outputs);

    Ok((
        vec![adx_line, dx_line, atr_line, tr_line],
        IndicatorState {
            state: state,
            inv_multiplier,
        },
    ))
}

/// Performs the main calculation loop for the ADX indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `period` - The period for the ADX calculation.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
/// * `adx_line` - A mutable reference to a vector for storing the ADX line.
/// * `output_vectors` - A mutable reference to an array of optional vectors for storing additional outputs.
//#[inline(always)]
fn cycle_adx(
    inputs: (&[f64], &[f64], &[f64]),
    inv_multiplier: f64,
    state: &mut State,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (adx_line, dx_line, atr_line, tr_line) = out_vecs;

    let (has_optional, want_dx, want_atr, want_tr) =
        crate::calc_want_flags!(dx_line, atr_line, tr_line);

    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (adx, dx, atr, tr) = calc(state, h, l, c);
        unsafe {
            *adx_line.get_unchecked_mut(i) = adx;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_dx, dx_line => dx,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, inv_multiplier)
            );
        }
    }
}

/// Calculates the current value of the Average Directional Index (ADX) indicator.
///
/// # Arguments
///
/// * `high` -  high price array.
/// * `low` - low price array.
/// * `adx` - The previous ADX value.
/// * `dmup` - The previous DM+ value.
/// * `dmdown` - The previous DM- value.
/// * `atr` - The previous ATR value.
/// * `period` - The period for the ADX calculation.
/// * `i` - The current index.
/// * `start` - The starting index for the calculation.
///
/// # Returns
///
/// A tuple containing the current ADX value, the current DX value, the current DM+ value, the current DM- value, the current ATR value, the current TR value, the updated DM+ value, and the updated DM- value.
#[inline(always)]
pub fn calc(state: &mut State, high: f64, low: f64, close: f64) -> (f64, f64, f64, f64) {
    let (dx, atr, tr) = current_dx(&mut state.dx_state, high, low, close);
    //state.adx += dx;
    //state.adx = state.adx * 0.2;
    state.adx = calc_wilders(state.adx, dx, state.multiplier);
    (state.adx, dx, atr, tr)
}
