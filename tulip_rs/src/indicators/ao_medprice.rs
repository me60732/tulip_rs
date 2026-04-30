use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::sma::{
    calc as sma_calc, multiplier as sma_multiplier, output_length as sma_output_length,
};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub use crate::indicators::ao::OPTIONS_WIDTH;
const SHORT_PERIOD: usize = 5;
const LONG_PERIOD: usize = 34;
/// Returns information about the Awesome Oscillator (AO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AO indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "ao",
        full_name: "Awesome Oscillator",
        indicator_type: IndicatorType::Momentum,
        display_type: DisplayType::Indicator,
        inputs: &["medprice"],
        options: &[],
        outputs: &["ao"],
        optional_outputs: &["short_sma", "long_sma"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_sum: f64,
    pub long_sum: f64,
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    medprice: Vec<f64>,
    multipliers: (f64, f64),
    state: State,
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.medprice.extend_from_slice(inputs[0]);

        let capacity = inputs[0].len();
        let mut ao_line = crate::uninit_vec!(f64, capacity);

        let (mut short_sma_line, mut long_sma_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &optional_outputs.unwrap_or(&[false, false]),
            short_sma_line: capacity,
            long_sma_line: capacity
        );

        cycle_ao(
            &self.medprice,
            self.multipliers,
            &mut self.state,
            &mut ao_line,
            (&mut short_sma_line, &mut long_sma_line),
        );
        self.medprice.drain(..self.medprice.len() - LONG_PERIOD);

        Ok(vec![ao_line, short_sma_line, long_sma_line])
    }
}
impl State {
    pub fn new(short_sum: f64, long_sum: f64) -> Self {
        State {
            short_sum,
            long_sum,
        }
    }
    pub fn init_state(medprice: &[f64], short_sma_line: &mut [f64]) -> Self {
        let mut state = Self::new(0.0, 0.0);
        let (multiplier, _) = multiplier((SHORT_PERIOD, LONG_PERIOD));
        for (i, &med_price) in medprice.iter().take(LONG_PERIOD).enumerate() {
            state.long_sum += med_price;
            let mut sma = 0.0;
            if i >= SHORT_PERIOD {
                sma = sma_calc(
                    &mut state.short_sum,
                    &med_price,
                    &medprice[i - SHORT_PERIOD],
                    &multiplier,
                );
            } else {
                state.short_sum += med_price;
            }
            crate::init_store_optional_outputs!(i, medprice.len(),
                short_sma_line => sma
            );
        }
        state
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the AO indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AO calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    35 // long_period
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `_options` - A slice containing the options for the AO calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Awesome Oscillator (AO) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high and low prices.
/// * `_options` - A slice containing the options for the AO calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the AO line.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_inputs(inputs, min_data(_options))?;

    let medprice = inputs[0];

    let (mut ao_line, (mut short_sma_line, mut long_sma_line)) = {
        let capacity = output_length(medprice.len(), _options);
        let short_capacity = sma_output_length(medprice.len(), &[SHORT_PERIOD as f64]);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &optional_outputs.unwrap_or(&[false, false]),
                short_sma_line: short_capacity,
                long_sma_line: capacity
            ),
        )
    };
    let multipliers = multiplier((SHORT_PERIOD, LONG_PERIOD));
    let mut state = State::init_state(medprice, &mut short_sma_line);

    let offset = crate::slice_outputs_start!(ao_line.len(), short_sma_line);
    cycle_ao(
        medprice,
        multipliers,
        &mut state,
        &mut ao_line,
        (&mut short_sma_line[offset..], &mut long_sma_line),
    );

    Ok((
        vec![ao_line, short_sma_line, long_sma_line],
        IndicatorState {
            state,
            multipliers,
            medprice: medprice[medprice.len() - LONG_PERIOD..].to_vec(),
        },
    ))
}

/// Performs the main calculation loop for the AO indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `short_period` - The short period for the AO calculation.
/// * `long_period` - The long period for the AO calculation.
/// * `prev_medprice` - A mutable reference to a deque containing the previous median prices.
/// * `start` - The starting index for the calculation.
/// * `ao_line` - A mutable reference to a vector for storing the AO line.
#[inline(always)]
fn cycle_ao(
    medprice: &[f64],
    multipliers: (f64, f64),
    state: &mut State,
    ao_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (short_sma_line, long_sma_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_sma_line, long_sma_line);

    for (j, i) in (LONG_PERIOD..medprice.len()).enumerate() {
        let (values, prev_values) = unsafe {
            (
                *medprice.get_unchecked(i),
                (
                    *medprice.get_unchecked(i - LONG_PERIOD),
                    *medprice.get_unchecked(i - SHORT_PERIOD),
                ),
            )
        };

        let (ao, short_sma, long_sma) = calc(state, values, prev_values, multipliers);
        unsafe { *ao_line.get_unchecked_mut(j) = ao };

        // Direct, inline storage - no function calls, no loops, no indirection
        // In your cycle_ao_simple function, replace the if has_optional block with:
        if has_optional {
            crate::store_optional_outputs!(j,
                want_short, short_sma_line => short_sma,
                want_long, long_sma_line => long_sma
            );
        }
    }
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    medprice: f64,
    prev_values: (f64, f64),
    multipliers: (f64, f64),
) -> (f64, f64, f64) {
    let (short_multiplier, long_multiplier) = multipliers;
    let (long_medprice, short_medprice) = prev_values;

    let short_sma = sma_calc(
        &mut state.short_sum,
        &medprice,
        &short_medprice,
        &short_multiplier,
    );
    let long_sma = sma_calc(
        &mut state.long_sum,
        &medprice,
        &long_medprice,
        &long_multiplier,
    );

    (short_sma - long_sma, short_sma, long_sma)
}

#[inline(always)]
pub fn multiplier(periods: (usize, usize)) -> (f64, f64) {
    (sma_multiplier(periods.0), sma_multiplier(periods.1))
}
