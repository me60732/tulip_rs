use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::multiplier as ema_multiplier;
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::kama_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::kama_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::kama_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::kama_simd::indicator_by_options as indicator;
}
/// Returns information about the Kaufman's Adaptive Moving Average (KAMA) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the KAMA indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "kama",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        full_name: "Kaufman's Adaptive Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["kama"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
    multipliers: (f64, f64),
    state: State,
}
impl IndicatorState {
    pub fn new(real: &[f64], period: usize, multipliers: (f64, f64), state: State) -> Self {
        Self {
            period,
            multipliers,
            state,
            real: real[real.len() - period - 1..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let mut kama_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        cycle_kama(
            &self.real,
            &mut self.state,
            self.period,
            self.multipliers,
            &mut kama_line,
        );
        self.real.drain(..self.real.len() - self.period - 1);

        Ok(vec![kama_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub kama: f64,
    pub sum: f64,
}
impl State {
    pub fn new(kama: f64, sum: f64) -> Self {
        Self { kama, sum }
    }
    pub fn init_state(real: &[f64], period: usize, kama_line: &mut [f64]) -> Self {
        let mut state = Self::new(
            real[period - 1],
            (1..period).map(|i| (real[i] - real[i - 1]).abs()).sum(),
        );
        let multipliers = multiplier();
        let values = unsafe {
            (
                real.get_unchecked(period),
                real.get_unchecked(period - 1),
                real.get_unchecked(0),
                &0.0,
            )
        };
        let kama = state.calc(values, multipliers, period, period);
        kama_line[0] = kama;

        state
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        values: (&f64, &f64, &f64, &f64),
        multipliers: (f64, f64),
        period: usize,
        i: usize,
    ) -> f64 {
        let (value, prev_value, last_value, old_value) = values;
        let (fast_ema, slow_ema) = multipliers;
        self.sum += (value - prev_value).abs();
        if i > period {
            self.sum -= (last_value - old_value).abs();
        }

        let efficiency_ratio = if self.sum != 0.0 {
            (value - last_value).abs() / self.sum
        } else {
            1.0
        };
        //let smoothing_constant = (efficiency_ratio * (fast_ema - slow_ema) + slow_ema).powi(2);
        let smoothing_constant = (fast_ema - slow_ema)
            .mul_add(efficiency_ratio, slow_ema)
            .powi(2);

        // Optimized calculation using C-style EMA pattern
        let per1 = 1.0 - smoothing_constant;
        //self.kama = self.kama * per1 + value * smoothing_constant;
        self.kama = self.kama.mul_add(per1, value * smoothing_constant);
        self.kama
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    if options[0] > 12.0 {
        let (short_multiplier, long_multiplier) = multiplier();
        let alpha = short_multiplier - long_multiplier;
        return min_process(
            options,
            Some((decimals, 0)),
            &[ema_multiplier(options[0] as usize).0, alpha],
            IndicatorInfoOrInteger::Info(&info()),
            min_data,
        );
    }
    min_process(
        options,
        Some((decimals, 0)),
        &[ema_multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Returns the minimum amount of data required for the KAMA indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the KAMA calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the KAMA calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Kaufman's Adaptive Moving Average (KAMA) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the KAMA calculation.
///
/// # Returns
///
/// A vector of vectors containing the KAMA line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier();

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut kama_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(real, period, &mut kama_line);
    // Perform the main KAMA calculation
    cycle_kama(real, &mut state, period, multipliers, &mut kama_line[1..]);

    Ok((
        vec![kama_line],
        IndicatorState::new(real, period, multipliers, state),
    ))
}

/// Calculates the Kaufman's Adaptive Moving Average (KAMA) from a previous state.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the KAMA calculation.
/// * `indicator_state` - A reference to the previous state containing the previous input values.
///
/// # Returns
///
/// A vector of vectors containing the KAMA line.

/// Performs the main calculation loop for the KAMA indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the KAMA calculation.
/// * `start` - The starting index for the calculation.
/// * `fast_period` - The fast period for the KAMA calculation.
/// * `slow_period` - The slow period for the KAMA calculation.
/// * `prev_real` - A mutable reference to a deque containing the previous input values.
/// * `kama_line` - A mutable reference to a vector for storing the KAMA line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
fn cycle_kama(
    real: &[f64],
    state: &mut State,
    period: usize,
    multipliers: (f64, f64),
    kama_line: &mut [f64],
) {
    //real.iter().enumerate().skip(start).for_each(|(i, value)| {
    for (j, i) in (period + 1..real.len()).enumerate() {
        let values = unsafe {
            (
                real.get_unchecked(i),
                real.get_unchecked(i - 1),
                real.get_unchecked(j+1),
                real.get_unchecked(j),
            )
        };
        let kama = state.calc(values, multipliers, period, i);
        //let kama = calc(values, state, multipliers, period, i);
        unsafe { *kama_line.get_unchecked_mut(j) = kama };
    }
}

/// Calculates the Kaufman's Adaptive Moving Average (KAMA) for the current data point using rolling sums.
///
/// # Arguments
///
/// * `sum` - The rolling sum of the input data.
/// * `weighted_sum` - The rolling weighted sum of the input data.
/// * `kama` - The previous KAMA value.
/// * `prev_real` - A reference to a deque containing the previous input values.
/// * `value` - The new value in the input data.
/// * `period` - The period for the KAMA calculation.
/// * `fast_ema` - The fast EMA smoothing constant.
/// * `slow_ema` - The slow EMA smoothing constant.
///
/// # Returns
///
/// The calculated KAMA, WMA, and SMA values.
#[inline(always)]
pub fn calc(
    state: &mut State,
    values: (&f64, &f64, &f64, &f64),
    multipliers: (f64, f64),
    period: usize,
    i: usize,
) -> f64 {
    state.calc(values, multipliers, period, i)
}

#[inline(always)]
pub fn multiplier() -> (f64, f64) {
    (ema_multiplier(2).0, ema_multiplier(30).0)
}
