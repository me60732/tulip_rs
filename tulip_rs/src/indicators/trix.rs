use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::dema::output_length as dema_output_length;
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::tema::{calc as tema_calc, output_length as tema_output_length};
pub use crate::indicators::tema::{multiplier, State};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trix_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trix_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::trix_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::trix_simd::indicator_by_options as indicator;
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

        let (mut trix_line, mut tema_line, mut dema_line, mut ema_line);
        {
            let capacity = inputs[0].len();
            trix_line = crate::uninit_vec!(f64, capacity);
            (tema_line, dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                tema_line: capacity,
                dema_line: capacity,
                ema_line: capacity
            );
        }
        cycle_trix(
            inputs[0],
            self.multipliers,
            &mut self.state,
            &mut trix_line,
            (&mut tema_line, &mut dema_line, &mut ema_line),
        );

        Ok(vec![trix_line, tema_line, dema_line, ema_line])
    }
}

/// Returns information about the TRIX indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about TRIX.
pub fn info() -> Info<'static> {
    Info {
        name: "trix",
        full_name: "Triple Exponential Oscillator (TRIX)",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trix"],
        optional_outputs: &["tema", "dema", "ema"],
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
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the TRIX indicator.
///
/// TRIX is built on TEMA so uses the same warm-up requirement.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the TRIX calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    let period = options[0] as usize;
    (period - 1) * 3 + 2
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TRIX calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Triple Exponential Oscillator (TRIX) indicator over the full input dataset.
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
///   `optional_outputs[0] = true` enables `tema`, `optional_outputs[1] = true` enables `dema`,
///   `optional_outputs[2] = true` enables `ema`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `trix`,
/// `outputs[1]` is `tema` (empty unless requested),
/// `outputs[2]` is `dema` (empty unless requested),
/// `outputs[3]` is `ema` (empty unless requested), and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let (mut trix_line, mut tema_line, mut dema_line, mut ema_line, mut state, real, multipliers);
    {
        let len = inputs[0].len();
        let capacity = output_length(len, options);
        let tema_cap = tema_output_length(len, options);
        let dema_cap = dema_output_length(len, options);
        let ema_cap = ema_output_length(len, options);

        // Initialize output storage: main TRIX line plus optional outputs (TEMA, DEMA, EMA)
        trix_line = crate::uninit_vec!(f64, capacity);
        (tema_line, dema_line, ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            tema_line: tema_cap,
            dema_line: dema_cap,
            ema_line: ema_cap
        );
        let period = options[0] as usize;
        state = init_state(
            inputs[0],
            period,
            capacity,
            (&mut tema_line, &mut dema_line, &mut ema_line),
        );
        let start = len - capacity;
        multipliers = multiplier(period);
        real = &inputs[0][start..]
    }
    let optional_outputs = {
        let offsets = crate::slice_outputs_start!(trix_line.len(), tema_line, dema_line, ema_line);
        (
            &mut tema_line[offsets.0..],
            &mut dema_line[offsets.1..],
            &mut ema_line[offsets.2..],
        )
    };

    cycle_trix(
        real,
        multipliers,
        &mut state,
        &mut trix_line,
        optional_outputs,
    );

    Ok((
        vec![trix_line, tema_line, dema_line, ema_line],
        IndicatorState::new(state, multipliers),
    ))
}

/// Performs the main calculation loop for the TRIX indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `multipliers` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
/// * `state` - A mutable reference to the current TEMA indicator state.
/// * `trix_line` - A mutable slice for storing the TRIX output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(tema_line, dema_line, ema_line)`.
fn cycle_trix(
    real: &[f64],
    multipliers: (f64, f64),
    state: &mut State,
    trix_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (tema_line, dema_line, ema_line) = out_vecs;
    let (has_optional, want_tema, want_dema, want_ema) =
        crate::calc_want_flags!(tema_line, dema_line, ema_line);

    for i in 0..real.len() {
        let (tema, dema, ema);
        unsafe {
            (*trix_line.get_unchecked_mut(i), tema, dema, ema) =
                calc(state, real.get_unchecked(i), multipliers)
        };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_tema, tema_line => tema,
                want_dema, dema_line => dema,
                want_ema, ema_line => ema
            );
        }
    }
}

/// Calculates TRIX for a single data point.
///
/// Updates the triple-smoothed EMA state and computes TRIX as the percentage rate of
/// change between the current and previous EMA3 value.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current TEMA indicator state.
/// * `value` - The current input data point.
/// * `multiplier` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
///
/// # Returns
///
/// A tuple `(trix, tema, dema, ema)` containing the current TRIX, TEMA, DEMA, and EMA values.
#[inline(always)]
pub fn calc(state: &mut State, value: &f64, multiplier: (f64, f64)) -> (f64, f64, f64, f64) {
    let prev_ema3 = state.ema3;
    let (tema, dema, ema) = tema_calc(state, value, multiplier);
    // Compute TRIX as percentage change if previous TEMA is non-zero.
    let trix = 100.0 * (state.ema3 - prev_ema3) / state.ema3;

    (trix, tema, dema, ema)
}

pub fn init_state(
    real: &[f64],
    period: usize,
    trix_capacity: usize,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) -> State {
    let remaining = real.len() - trix_capacity;
    let (tema_line, dema_line, ema_line) = out_vecs;
    let tema_capacity = tema_output_length(real.len(), &[period as f64]);
    let mut state = State::init_state(real, period, tema_capacity, (dema_line, ema_line));
    let mut i = real.len() - tema_capacity;
    let multiplier = multiplier(period);

    while i < remaining {
        let value = &real[i];
        let (tema, dema, ema) = tema_calc(&mut state, value, multiplier);

        crate::init_store_optional_outputs!(i, real.len(),
            tema_line => tema,
            dema_line => dema,
            ema_line => ema
        );
        i += 1;
    }
    state
}
