use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::stddev::multiplier;
use crate::indicators::stddev::{calc as stddev_calc, State as StddevState};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options as indicator;
}
const ANNUAL: f64 = 15.874507866387544; // 252_f64.sqrt()

pub fn info() -> Info<'static> {
    Info {
        name: "volatility",
        full_name: "Volatility Indicator",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Volatility,
        inputs: &["real"],
        options: &["period"],
        outputs: &["volatility"],
        optional_outputs: &[],
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: f64,
}
impl IndicatorState {
    pub fn new(state: State, multiplier: f64) -> Self {
        Self {
            state,
            multiplier
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

        let mut volatility_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            self.multiplier,
            &mut self.state,
            &mut volatility_line,
        );

        Ok(vec![volatility_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub stddev_state: StddevState,
    pub prev_real: f64,
}
impl State {
    pub fn new(prev_real: f64, period: usize) -> Self {
        let stddev_state = StddevState::new(0.0, 0.0);
        let buffer = Buffer::new(period);
        State {
            prev_real,
            stddev_state,
            buffer,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let (mut sum, mut sum_sq) = (0.0, 0.0);
        let mut buffer = Buffer::new(period);
        for i in 1..=period {
            let v = real[i] / real[i - 1] - 1.0;
            buffer.push(v);
            sum += v;
            sum_sq += v * v;
        }

        Self {
            stddev_state: StddevState::new(sum, sum_sq),
            buffer,
            prev_real: real[period],
        }
    }
    #[inline(always)]
    pub fn calc(&mut self, real: f64, multiplier: f64) -> f64 {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info(value).unwrap();
        let (sd, _) = stddev_calc(&mut self.stddev_state, &value, &prev_value, multiplier);
        sd * ANNUAL
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, real: f64, multiplier: f64) -> f64 {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info_unchecked(value);
        let (sd, _) = stddev_calc(&mut self.stddev_state, &value, &prev_value, multiplier);
        sd * ANNUAL
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum required data points (equal to the period).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 2
}

/// Returns the output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Full-indicator calculation for Volatility.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    
    validate_inputs(inputs, min_data(options))?;
    let mut vol_line = {
        let capacity = output_length(inputs[0].len(), options);
        crate::uninit_vec!(f64, capacity)
    };
    let mut state = State::init_state(inputs[0], period);

    cycle(&inputs[0][period+1..], multiplier, &mut state, &mut vol_line);

    Ok((vec![vol_line], IndicatorState { multiplier, state }))
}
/// Loop through the data calling calc() for each bar.
/// Parameters:
/// - real: full data slice.
/// - start: starting index (>= period).
/// - period: period used for stddev calculation.
/// - multiplier: the multiplier from stddev_multiplier.
/// - vol: previous volatility value.
/// - sum, sum_sq: rolling state for stddev calculation.
fn cycle(real: &[f64], multiplier: f64, state: &mut State, vol_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *vol_line.get_unchecked_mut(i) =
                state.calc_unchecked(*real.get_unchecked(i), multiplier);
        }
    }
}

/// Calculation for a single bar of Volatility.
/// All per‑bar math (including calling stddev_calc) is done here.
/// Parameters:
/// - real: full data slice.
/// - i: current index (must be at least period).
/// - period: period for stddev.
/// - multiplier: multiplier from stddev_multiplier.
/// - sum, sum_sq: rolling state for stddev.
///
/// Returns a tuple:
///     (volatility, new_sum, new_sum_sq, sma)
#[inline(always)]
pub fn calc(state: &mut State, real: f64, multiplier: f64) -> f64 {
    state.calc(real, multiplier)
}
#[inline(always)]
pub unsafe fn calc_unchecked(state: &mut State, real: f64, multiplier: f64) -> f64 {
    state.calc_unchecked(real, multiplier)
}
