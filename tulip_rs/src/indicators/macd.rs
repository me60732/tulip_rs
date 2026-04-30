use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::{
    calc as calc_ema, multiplier as ema_multiplier, output_length as ema_output_length,
};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 3;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::macd_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::macd_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::macd_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::macd_simd::indicator_by_options as indicator;
}

pub fn info() -> Info<'static> {
    Info {
        name: "macd",
        full_name: "Moving Average Convergence Divergence",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["short_period", "long_period", "signal_period"],
        outputs: &["macd_line", "signal_line", "histogram"],
        optional_outputs: &["short_ema", "long_ema"],
    }
}
#[derive(Default, Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64), (f64, f64), (f64, f64)),
    state: State,
}
impl IndicatorState {
    pub fn new(multipliers: ((f64, f64), (f64, f64), (f64, f64)), state: State) -> Self {
        Self { multipliers, state }
    }
}
impl TIndicatorState<1> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut macd_line, mut signal_line, mut histogram, mut short_ema_line, mut long_ema_line);
        {
            let capacity = inputs[0].len();

            // Pre-allocate the result vectors with the calculated capacities
            macd_line = crate::uninit_vec!(f64, capacity);
            signal_line = crate::uninit_vec!(f64, capacity);
            histogram = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
            short_ema_line: capacity,
            long_ema_line: capacity
            );
        }
        cycle_macd(
            inputs[0],
            self.multipliers,
            &mut self.state,
            (&mut macd_line, &mut signal_line, &mut histogram),
            (&mut short_ema_line, &mut long_ema_line),
        );
        Ok(vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ])
    }
}
#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub short_ema: f64,
    pub long_ema: f64,
    pub signal: f64,
}
impl State {
    pub fn new(short_ema: f64, long_ema: f64, signal: f64) -> Self {
        Self {
            short_ema,
            long_ema,
            signal,
        }
    }
    pub fn init_state(
        real: &[f64],
        periods: (usize, usize, usize),
        multipliers: ((f64, f64), (f64, f64), (f64, f64)),
        macd_line: &mut [f64],
        out_vecs: (&mut [f64], &mut [f64]),
    ) -> Self {
        let (_, long_period, signal_period) = periods;
        let mut state = Self::new(real[0], real[0], 0.0);
        let (short_ema_line, long_ema_line) = out_vecs;
        let (has_optional, _, _) = crate::calc_want_flags!(short_ema_line, long_ema_line);
        let mut count = 0;
        for i in 1..long_period + signal_period - 2 {
            // was -2
            let (macd, _, _) = calc(&mut state, &real[i], multipliers);
            if i == long_period - 1 {
                state.signal = macd;
            }
            if i >= long_period - 1 {
                macd_line[count] = macd;
                count += 1;
            }
            if has_optional {
                crate::init_store_optional_outputs!(i, real.len(),
                    short_ema_line => state.short_ema,
                    long_ema_line => state.long_ema
                );
            }
        }

        state
    }
}
pub fn output_length(data_len: usize, options: &[f64]) -> (usize, usize, usize) {
    //let min_data = min_data(&options);
    let long_period = options[1] as usize;
    let signal_period = options[2] as usize;

    let macd_capacity = data_len - long_period + 1;
    let signal_capacity = macd_capacity - signal_period + 1;
    let histogram_capacity = signal_capacity;
    (macd_capacity, signal_capacity, histogram_capacity)
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let (_short_multiplier, long_multiplier, _signal_multiplier) = multiplier(
        options[0] as usize,
        options[1] as usize,
        options[2] as usize,
    );
    min_process(
        options,
        Some((decimals, 0)),
        &[long_multiplier.0],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
#[inline]
pub fn min_data(options: &[f64]) -> usize {
    (options[1] + options[2]) as usize - 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] || options[2] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let (
        mut macd_line,
        mut signal_line,
        mut histogram,
        mut short_ema_line,
        mut long_ema_line,
        multipliers,
        mut state,
        real,
    );
    {
        let short_period = options[0] as usize;
        let long_period = options[1] as usize;
        let signal_period = options[2] as usize;

        multipliers = multiplier(short_period, long_period, signal_period);
        // Calculate capacities
        let len = inputs[0].len();
        let (macd_capacity, signal_capacity, histogram_capacity) = output_length(len, options);

        let short_ema_capacity = ema_output_length(len, &[short_period as f64]);
        let long_ema_capacity = ema_output_length(len, &[long_period as f64]);
        // Pre-allocate the result vectors with the calculated capacities
        macd_line = crate::uninit_vec!(f64, macd_capacity);
        signal_line = crate::uninit_vec!(f64, signal_capacity);
        histogram = crate::uninit_vec!(f64, histogram_capacity);

        (short_ema_line, long_ema_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: long_ema_capacity
        );
        state = State::init_state(
            inputs[0],
            (short_period, long_period, signal_period),
            multipliers,
            &mut macd_line,
            (&mut short_ema_line, &mut long_ema_line),
        );
        let start = long_period + signal_period - 2;
        real = &inputs[0][start..]
    }
    let (macd_offset, short_offset, long_offset) =
        crate::slice_outputs_start!(signal_line.len(), macd_line, short_ema_line, long_ema_line);
    cycle_macd(
        real,
        multipliers,
        &mut state,
        (
            &mut macd_line[macd_offset..],
            &mut signal_line,
            &mut histogram,
        ),
        (
            &mut short_ema_line[short_offset..],
            &mut long_ema_line[long_offset..],
        ),
    );

    Ok((
        vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ],
        IndicatorState::new(multipliers, state),
    ))
}

//#[inline(always)]
fn cycle_macd(
    real: &[f64],
    multipliers: ((f64, f64), (f64, f64), (f64, f64)),
    state: &mut State,
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (macd_line, signal_line, histogram_line) = outputs;

    let (short_ema_line, long_ema_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        unsafe {
            (
                *macd_line.get_unchecked_mut(i),
                *signal_line.get_unchecked_mut(i),
                *histogram_line.get_unchecked_mut(i),
            ) = calc(state, real.get_unchecked(i), multipliers);
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => state.short_ema,
                want_long, long_ema_line => state.long_ema
            );
        }
    }
}

/// Calculates the current MACD value.
///
/// # Arguments
///
/// * `value` - The current price value.
/// * `short_ema` - The current short period EMA value.
/// * `long_ema` - The current long period EMA value.
/// * `signal` - The current signal line value.
/// * `short_period` - The short period for the MACD calculation.
/// * `long_period` - The long period for the MACD calculation.
/// * `signal_period` - The signal period for the MACD calculation.
///
/// # Returns
///
/// A tuple containing the updated short period EMA, long period EMA, and signal line values.
#[inline(always)]
pub fn calc(
    state: &mut State,
    value: &f64,
    multipliers: ((f64, f64), (f64, f64), (f64, f64)),
) -> (f64, f64, f64) {

    let (short_multiplier, long_multiplier, signal_multiplier) = multipliers;
    state.short_ema = calc_ema(value, state.short_ema, short_multiplier);
    state.long_ema = calc_ema(value, state.long_ema, long_multiplier);

    let macd_value = state.short_ema - state.long_ema;
    state.signal = calc_ema(&macd_value, state.signal, signal_multiplier);

    (macd_value, state.signal, macd_value - state.signal)
}

pub fn multiplier(
    short_period: usize,
    long_period: usize,
    signal_period: usize,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    (
        ema_multiplier(short_period),
        ema_multiplier(long_period),
        ema_multiplier(signal_period),
    )
}
