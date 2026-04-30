use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::dema::output_length as dema_output_length;
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::tema::{calc as tema_calc, output_length as tema_output_length};
pub use crate::indicators::tema::{multiplier, State};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trix_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trix_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::trix_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
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
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trix"],
        optional_outputs: &["tema", "dema", "ema"],
    }
}
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

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - Options for the TRIX calculation.
/// * `recent_only` - Option for computing only the most recent values.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the TRIX indicator for an entire dataset.
///
/// # Arguments
///
/// * `inputs` - A slice containing the input data vectors.
/// * `options` - A slice containing the options for the TRIX calculation.
/// * `recent_only` - Option to calculate only the most recent values.
/// * `optional_outputs` - Optional slice indicating which additional outputs to produce.
///
/// # Returns
///
/// An `Output` struct containing the TRIX line and, if requested, the additional outputs.
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

/// Calculates the TRIX indicator from the previous state.
///
/// This function uses the previous state (stored EMA/tema values) and processes the new data points
/// to update and extend the TRIX calculation.
///
/// # Arguments
///
/// * `inputs` - A slice containing the input data vectors.
/// * `options` - A slice containing the options for the TRIX calculation.
/// * `indicator_state` - An `IndicatorState` struct containing prior state information.
/// * `optional_outputs` - Optional slice indicating which additional outputs to produce.
///
/// # Returns
///
/// An `Output` struct containing the updated TRIX line and relevant state.

/// Performs the main calculation loop for the TRIX indicator.
///
/// This function closely mirrors the structure of TEMA's cycle routine. It iterates over
/// the input data starting at `start` and updates the underlying EMAs and TEMA in a single pass.
/// At each step, it computes TRIX and pushes it to the main output vector as well as the optional outputs.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for TRIX.
/// * `start` - The starting index for the calculation.
/// * `trix_line` - A mutable reference to the main TRIX output vector.
/// * `prev_tema` - The previous TEMA value (used for the rate of change calculation).
/// * `prev_ema1` - The previous EMA1 value.
/// * `prev_ema2` - The previous EMA2 value.
/// * `prev_ema3` - The previous EMA3 value.
/// * `output_vectors` - A mutable slice of optional output vectors (for tema, dema, ema).
///
/// # Returns
///
/// A tuple containing the updated state:
/// `(trix, tema, dema, ema1)`
/// where `trix` is the last TRIX value computed.
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

/// Calculates TRIX for a single data point in one pass.
///
/// It first calls the TEMA calc function to update the triple-smoothed EMA values and then computes TRIX
/// as the percentage rate of change between the current and previous TEMA.
///
/// # Arguments
///
/// * `value` - The current data point.
/// * `prev_tema` - Previous TEMA value (used for rate of change).
/// * `prev_ema1` - Previous EMA1 value.
/// * `prev_ema2` - Previous EMA2 value.
/// * `prev_ema3` - Previous EMA3 value.
/// * `multiplier` - Multiplier computed from the period.
///
/// # Returns
///
/// A tuple containing:
/// 1. `trix` - The current TRIX value.
/// 2. `tema` - The current TEMA value.
/// 3. `dema` - The current DEMA value.
/// 4. `ema1` - The updated EMA1.
/// 5. `ema2` - The updated EMA2.
/// 6. `ema3` - The updated EMA3.
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
