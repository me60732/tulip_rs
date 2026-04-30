use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::{
    calc as calc_ema, multiplier as ema_multiplier, output_length as ema_output_length,
};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 4;
pub const OPTIONS_WIDTH: usize = 2;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::kvo_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::kvo_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::kvo_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::kvo_simd::indicator_by_options as indicator;
}

/// Returns information about the Klinger Volume Oscillator (KVO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the KVO indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "kvo",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Volume,
        full_name: "Klinger Volume Oscillator",
        inputs: &["high", "low", "close", "volume"],
        options: &["short_period", "long_period"],
        outputs: &["kvo"],
        optional_outputs: &["short_ema", "long_ema"],
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: ((f64, f64), (f64, f64)),
}
impl IndicatorState {
    pub fn new(multipliers: ((f64, f64), (f64, f64)), state: State) -> Self {
        Self { multipliers, state }
    }
}
impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut kvo_line, mut short_ema_line, mut long_ema_line);
        {
            let capacity = inputs[0].len();
            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: capacity,
                long_ema_line: capacity
            );

            kvo_line = crate::uninit_vec!(f64, capacity);
        }
        cycle_kvo(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            self.multipliers,
            &mut kvo_line,
            &mut self.state,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![kvo_line, short_ema_line, long_ema_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_ema: f64,
    pub long_ema: f64,
    pub cm: f64,
    pub trend: f64,
    pub prev_hlc: f64,
    pub prev_high: f64,
    pub prev_low: f64,
}
impl State {
    pub fn new(
        short_ema: f64,
        long_ema: f64,
        trend: f64,
        cm: f64,
        prev_hlc: f64,
        prev_high: f64,
        prev_low: f64,
    ) -> Self {
        Self {
            short_ema,
            long_ema,
            trend,
            cm,
            prev_hlc,
            prev_high,
            prev_low,
        }
    }
    pub fn init_state(
        inputs: (&[f64], &[f64], &[f64], &[f64]),
        kvo_line: &Vec<f64>,
        periods: (usize, usize),
        short_ema_line: &mut [f64],
    ) -> Self {
        let capacity = kvo_line.capacity();
        let (high, low, close, volume) = inputs;
        let output_start = high.len() - capacity;
        let mut state = Self::new(
            0.0,
            0.0,
            -2.0,
            0.0,
            high[0] + low[0] + close[0],
            high[0],
            low[0],
        );
        let (short_period, long_period) = periods;
        let (short_multiplier, long_multiplier) = multiplier(short_period, long_period);
        for i in 1..output_start {
            let inputs = unsafe {
                (
                    *high.get_unchecked(i),
                    *low.get_unchecked(i),
                    *close.get_unchecked(i),
                    *volume.get_unchecked(i),
                )
            };

            let vf = calc_vf(&mut state, inputs);
            if i == 1 {
                // Initialize EMAs only once, just like C
                state.short_ema = vf;
                state.long_ema = vf;
            } else {
                // Use normal EMA calculation for subsequent points
                state.short_ema = calc_ema(&vf, state.short_ema, short_multiplier);
                state.long_ema = calc_ema(&vf, state.long_ema, long_multiplier);
            }
            crate::init_store_optional_outputs!(i, high.len(),
                short_ema_line => state.short_ema
            );
        }

        state
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let multipliers = multiplier(options[0] as usize, options[1] as usize);
    min_process(
        options,
        Some((decimals, 0)),
        &[multipliers.1 .0],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the KVO indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the KVO calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize + 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the KVO calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Klinger Volume Oscillator (KVO) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the KVO calculation.
///
/// # Returns
///
/// A vector of vectors containing the KVO line.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;
    let [high, low, close, volume] = inputs;

    let (mut kvo_line, mut short_ema_line, mut long_ema_line, mut state, multipliers, inputs);
    {
        let capacity = output_length(high.len(), options);
        let short_capacity = ema_output_length(high.len(), &[options[0]]);
        kvo_line = crate::uninit_vec!(f64, capacity);

        (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: short_capacity,
            long_ema_line: capacity
        );
        let short_period = options[0] as usize;
        let long_period = options[1] as usize;
        multipliers = multiplier(short_period, long_period);
        // Perform the main KVO calculation
        state = State::init_state(
            (&high, &low, &close, &volume),
            &kvo_line,
            (short_period, long_period),
            &mut short_ema_line,
        );
        let from = high.len() - capacity;
        inputs = (&high[from..], &low[from..], &close[from..], &volume[from..])
    }
    let optional_outputs = {
        let offset = crate::slice_outputs_start!(kvo_line.len(), short_ema_line);
        (&mut short_ema_line[offset..], long_ema_line.as_mut_slice())
    };
    
    cycle_kvo(
        inputs,
        multipliers,
        &mut kvo_line,
        &mut state,
        optional_outputs,
    );

    Ok((
        vec![kvo_line, short_ema_line, long_ema_line],
        IndicatorState { multipliers, state },
    ))
}

/// Calculates the Klinger Volume Oscillator (KVO) from a previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the KVO calculation.
/// * `indicator_state` - A reference to the previous state containing the previous input values.
///
/// # Returns
///
/// A vector of vectors containing the KVO line.

/// Performs the main calculation loop for the KVO indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `volume` - A slice of volume data.
/// * `short_period` - The short period for the KVO calculation.
/// * `long_period` - The long period for the KVO calculation.
/// * `kvo_line` - A mutable reference to a vector for storing the KVO line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
/// * `prev_state` - An optional tuple containing the previous state values.
fn cycle_kvo(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    multipliers: ((f64, f64), (f64, f64)),
    kvo_line: &mut [f64],
    state: &mut State,
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (high, low, close, volume) = inputs;
    let (short_ema_line, long_ema_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            )
        };
        let kvo = calc(state, inputs, multipliers);
        unsafe { *kvo_line.get_unchecked_mut(i) = kvo };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => state.short_ema,
                want_long, long_ema_line => state.long_ema
            );
        }
    }
}

/// Calculates the Klinger Volume Oscillator (KVO) for the current data point.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `volume` - The current volume data.
/// * `prev_high` - The previous high price.
/// * `prev_low` - The previous low price.
/// * `prev_close` - The previous close price.
/// * `prev_volume` - The previous volume data.
/// * `short_ema` - The previous short EMA value.
/// * `long_ema` - The previous long EMA value.
/// * `short_period` - The short period for the KVO calculation.
/// * `long_period` - The long period for the KVO calculation.
/// * `trend` - The previous trend value.
/// * `cm` - The previous cm value.
///
/// # Returns
///
/// A tuple containing the calculated KVO, new short EMA, new long EMA, new trend, and new cm values.
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (f64, f64, f64, f64),
    multipliers: ((f64, f64), (f64, f64)),
) -> f64 {
    // Extract multipliers once (minor optimization)

    let vf = calc_vf(state, inputs);
    let (short_multiplier, long_multiplier) = multipliers;
    state.short_ema = calc_ema(&vf, state.short_ema, short_multiplier);
    state.long_ema = calc_ema(&vf, state.long_ema, long_multiplier);
    state.short_ema - state.long_ema
}

#[inline(always)]
pub(crate) fn calc_vf(state: &mut State, inputs: (f64, f64, f64, f64)) -> f64 {
    let (high, low, close, volume) = inputs;

    let hlc = high + low + close;
    let dm = high - low;

    // Update trend and cm
    if state.trend != 1.0 && hlc > state.prev_hlc {
        state.trend = 1.0;
        state.cm = state.prev_high - state.prev_low;
    } else if state.trend != -1.0 && hlc < state.prev_hlc {
        state.trend = -1.0;
        state.cm = state.prev_high - state.prev_low;
    }
    state.cm += dm.max(f64::EPSILON);

    state.prev_hlc = hlc;
    state.prev_high = high;
    state.prev_low = low;

    (dm / state.cm).mul_add(2.0, -1.0).abs() * volume * 100.0 * state.trend
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}
