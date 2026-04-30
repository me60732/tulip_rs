use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::md::multiplier;
use crate::indicators::md::{calc_md, calc_md_simd, output_length as md_output_length};
use crate::indicators::sma::calc as calc_sma;
use crate::indicators::typprice::calc as typprice_calc;
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::cci_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::cci_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::cci_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::cci_simd::indicator_by_options as indicator;
}

/// Returns information about the Commodity Channel Index (CCI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the CCI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "cci",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        full_name: "Commodity Channel Index",
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["cci"],
        optional_outputs: &["sma", "md", "typprice"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub sum: f64,
}
impl State {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State {
        let (sma_line, md_line, typprice_line) = out_vecs;
        let mut state = Self {
            buffer: Buffer::new(period),
            sum: 0.0,
        };
        let (mut sma, mut md) = (0.0, 0.0);
        let mut typprice;
        for (i, ((high_val, low_val), close_val)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .take(period * 2 - 2)
        {
            if i < period {
                typprice = typprice_calc(high_val, low_val, close_val);
                state.buffer.push(typprice);
                state.sum += typprice;
            } else {
                (_, sma, md, typprice) =
                    calc(&mut state, high_val, low_val, close_val, multiplier(period));
            }

            crate::init_store_optional_outputs!(i, high.len(),
                sma_line => sma,
                md_line => md,
                typprice_line => typprice
            );
        }
        state
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, multiplier: f64, period: usize) -> Self {
        Self {
            state,
            multiplier,
            period,
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

        let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let capacity = inputs[0].len();
            (typprice_line, sma_line, md_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                typprice_line: capacity,
                sma_line: capacity,
                md_line: capacity
            );
            cci_line = crate::uninit_vec!(f64, capacity);
        };

        if self.period > 20 {
            cycle::<true>(
                (inputs[0], inputs[1], inputs[2]),
                self.multiplier,
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            );
        } else {
            cycle::<false>(
                (inputs[0], inputs[1], inputs[2]),
                self.multiplier,
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            );
        }

        Ok(vec![cci_line, sma_line, md_line, typprice_line])
    }
}

pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the CCI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the CCI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize * 2 - 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the CCI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Commodity Channel Index (CCI) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, and close prices.
/// * `options` - A slice containing the options for the CCI calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the CCI line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
    {
        let capacity = output_length(high.len(), options);
        let md_capacity = md_output_length(high.len(), options);
        cci_line = crate::uninit_vec!(f64, capacity);
        (sma_line, md_line, typprice_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            sma_line: md_capacity,
            md_line: md_capacity,
            typprice_line: high.len()
        );
    };

    let mut state = State::init_state(
        high,
        low,
        close,
        period,
        (&mut sma_line, &mut md_line, &mut typprice_line),
    );
    let optional_outputs = {
        let offset = crate::slice_outputs_start!(cci_line.len(), sma_line, md_line, typprice_line);
        (
            &mut sma_line[offset.0..],
            &mut md_line[offset.1..],
            &mut typprice_line[offset.2..],
        )
    };
    let inputs = {
        let from = period * 2 - 2;
        (&high[from..], &low[from..], &close[from..])
    };
    if period > 20 {
        cycle::<true>(
            inputs,
            multiplier,
            &mut state,
            &mut cci_line,
            optional_outputs,
        );
    } else {
        cycle::<false>(
            inputs,
            multiplier,
            &mut state,
            &mut cci_line,
            optional_outputs
        );
    }

    Ok((
        vec![cci_line, sma_line, md_line, typprice_line],
        IndicatorState::new(state, multiplier, period),
    ))
}

/// Performs the main calculation loop for the CCI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `period` - The period for the CCI calculation.
/// * `cci_line` - A mutable reference to a vector for storing the CCI line.
/// * `output_vectors` - A mutable reference to a slice of optional output vectors.
fn cycle<const SIMD: bool>(
    inputs: (&[f64], &[f64], &[f64]),
    multiplier: f64,
    buffer: &mut State,
    cci_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (sma_line, md_line, typprice_line) = out_vecs;
    let (has_optional, want_typ, want_sma, want_md) =
        crate::calc_want_flags!(typprice_line, sma_line, md_line);

    //high.iter().zip(low.iter()).zip(close.iter()).skip(start).enumerate().for_each(|(i, ((h, l), c))| {
    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            )
        };
        let (cci, sma, md, typprice);
        if SIMD {
            (cci, sma, md, typprice) = unsafe { calc_unchecked_simd(buffer, h, l, c, multiplier) };
        } else {
            (cci, sma, md, typprice) = unsafe { calc_unchecked(buffer, h, l, c, multiplier) };
        }

        unsafe { *cci_line.get_unchecked_mut(i) = cci };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_sma, sma_line => sma,
                want_md, md_line => md,
                want_typ, typprice_line => typprice
            );
        }
    }
}
/// Calculates the current Commodity Channel Index (CCI) value.
///
/// # Arguments
///
/// * `high` - The high price.
/// * `low` - The low price.
/// * `close` - The close price.
/// * `prev_typprice` - A reference to a deque containing the previous typical prices.
/// * `sum` - The sum of the previous typical prices.
/// * `period` - The period for the CCI calculation.
///
/// # Returns
///
/// The CCI value, the mean deviation, the updated sum, the SMA, and the typical price.
#[inline(always)]
pub fn calc(
    state: &mut State,
    high: &f64,
    low: &f64,
    close: &f64,
    multiplier: f64,
) -> (f64, f64, f64, f64) {
    let typprice = typprice_calc(high, low, close);
    //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);

    if let Some(old) = state.buffer.push_with_info(typprice) {
        let sma = calc_sma(&mut state.sum, &typprice, &old, &multiplier);
        let md = calc_md(state.buffer.get_slice(), sma, multiplier);

        let cci = (typprice - sma) / (0.015 * md);
        return (cci, sma, md, typprice);
    }

    state.sum += typprice;
    (0.0, 0.0, 0.0, typprice)
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked(
    state: &mut State,
    high: &f64,
    low: &f64,
    close: &f64,
    multiplier: f64,
) -> (f64, f64, f64, f64) {
    let typprice = typprice_calc(high, low, close);

    let old = state.buffer.push_with_info_unchecked(typprice);

    let sma = calc_sma(&mut state.sum, &typprice, &old, &multiplier);
    let md = calc_md(state.buffer.get_slice(), sma, multiplier);

    let cci = (typprice - sma) / (0.015 * md);
    (cci, sma, md, typprice)
}
/// calc using simd
#[inline(always)]
pub(crate) unsafe fn calc_unchecked_simd(
    state: &mut State,
    high: &f64,
    low: &f64,
    close: &f64,
    multiplier: f64,
) -> (f64, f64, f64, f64) {
    let typprice = typprice_calc(high, low, close);
    //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);
    let old = state.buffer.push_with_info_unchecked(typprice);
    let sma = calc_sma(&mut state.sum, &typprice, &old, &multiplier);

    let md = calc_md_simd::<4>(state.buffer.get_slice(), sma, multiplier);

    let cci = (typprice - sma) / (0.015 * md);
    (cci, sma, md, typprice)
}
