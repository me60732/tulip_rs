use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::ring_buffer::multi_buffer::multi_buffer::{MultiBuffer as Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
//use wide::*;
use std::simd::{num::SimdFloat, Simd};
pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 3;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ultosc_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ultosc_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::ultosc_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::ultosc_simd::indicator_by_options as indicator;
}
const MULTIPLIERS: Simd<f64, 2> = Simd::from_array([4.0, 2.0]);
/// Returns meta-information for this indicator.
/// Adjust the fields (name, full_name, inputs, options, etc.) as needed.
pub fn info() -> Info<'static> {
    Info {
        name: "ultosc",
        full_name: "Ultimate Oscillator",
        indicator_type: IndicatorType::Momentum,
        display_type: DisplayType::Indicator,
        // Inputs are expected to be: high, low, close
        inputs: &["high", "low", "close"],
        // Options: short_period, medium_period, long_period
        options: &["short_period", "medium_period", "long_period"],
        outputs: &["ultosc"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(state: State, periods: (usize, usize)) -> Self {
        Self { state, periods }
    }
}

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut ultosc_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            inputs[1],
            inputs[2],
            self.periods,
            &mut self.state,
            &mut ultosc_line,
        );

        Ok(vec![ultosc_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer<2>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub bp_sums_2x: Simd<f64, 2>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub tr_sums_2x: Simd<f64, 2>,
    pub bp_long_sum: f64,
    pub tr_long_sum: f64,
    pub prev_close: f64,
}
// Custom serialization functions
fn serialize_f64x2<S>(data: &Simd<f64, 2>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    data.to_array().serialize(serializer)
}

fn deserialize_f64x2<'de, D>(deserializer: D) -> Result<Simd<f64, 2>, D::Error>
where
    D: Deserializer<'de>,
{
    let array = <[f64; 2]>::deserialize(deserializer)?;
    Ok(Simd::from_array(array))
}

impl State {
    pub fn new(long_period: usize, prev_close: f64) -> Self {
        Self {
            buffer: Buffer::new(long_period),
            bp_long_sum: 0.0,
            bp_sums_2x: Simd::<f64, 2>::splat(0.0),
            tr_long_sum: 0.0,
            tr_sums_2x: Simd::<f64, 2>::splat(0.0),
            prev_close,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        periods: (usize, usize, usize),
        ultosc_line: &mut [f64],
    ) -> Self {
        let long_period = periods.2;
        let mut state = Self::new(long_period, close[0]);
        for (i, ((high_val, low_val), close_val)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .skip(1)
            .take(long_period)
        {
            let ult = state.calc(high_val, low_val, close_val, (periods.0, periods.1));
            if i == long_period {
                ultosc_line[0] = ult;
            }
        }
        state
    }

    #[inline(always)]
    pub fn calc(&mut self, high: &f64, low: &f64, close: &f64, periods: (usize, usize)) -> f64 {
        const DIV: f64 = 100.0 / 7.0;
        let (short_period, medium_period) = periods;

        let true_low = low.min(self.prev_close);
        let true_high = high.max(self.prev_close);
        let bp = close - true_low;
        let tr = true_high - true_low;

        if let Some(old) = self.buffer.push_with_info([bp, tr]) {
            self.bp_long_sum += bp - old[0];
            self.tr_long_sum += tr - old[1];
        } else {
            self.bp_long_sum += bp;
            self.tr_long_sum += tr;
        }

        let (bp_x2, tr_x2) = (Simd::<f64, 2>::splat(bp), Simd::<f64, 2>::splat(tr));
        let (bp_r, tr_r) = {
            let [bp, tr] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            (
                Simd::<f64, 2>::from_array(bp),
                Simd::<f64, 2>::from_array(tr),
            )
        };

        self.bp_sums_2x += bp_x2 - bp_r;
        self.tr_sums_2x += tr_x2 - tr_r;
        self.prev_close = *close;

        if self.buffer.is_full() {
            let weight_sum = (MULTIPLIERS * self.bp_sums_2x / self.tr_sums_2x).reduce_sum();
            //let weight_sum = first_second.reduce_add();
            let third = self.bp_long_sum / self.tr_long_sum;
            return (weight_sum + third) * DIV; // did originally .max(0.0) this
        }
        0.0
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        periods: (usize, usize),
    ) -> f64 {
        const DIV: f64 = 100.0 / 7.0;

        let (short_period, medium_period) = periods;
        let true_low = low.min(self.prev_close);
        let true_high = high.max(self.prev_close);
        let bp = close - true_low;
        let tr = true_high - true_low;

        let old = self.buffer.push_with_info_unchecked([bp, tr]);
        self.bp_long_sum += bp - old[0];
        self.tr_long_sum += tr - old[1];

        let (bp_x2, tr_x2) = (Simd::<f64, 2>::splat(bp), Simd::<f64, 2>::splat(tr));
        let (bp_r, tr_r) = {
            let [bp, tr] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            (
                Simd::<f64, 2>::from_array(bp),
                Simd::<f64, 2>::from_array(tr),
            )
        };

        self.bp_sums_2x += bp_x2 - bp_r;
        self.tr_sums_2x += tr_x2 - tr_r;

        let weight_sum = (MULTIPLIERS * self.bp_sums_2x / self.tr_sums_2x).reduce_sum();
        //let weight_sum = first_second.reduce_add();
        let third = self.bp_long_sum / self.tr_long_sum;
        self.prev_close = close;
        (weight_sum + third) * DIV
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required by the indicator.
/// This is often simply the period.
pub fn min_data(options: &[f64]) -> usize {
    options[2] as usize + 1
}
/// Determines the length of the output given the data and recent-only parameter.
/// You can adjust how the calculation lag is handled here.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] < options[0] || options[2] < options[1] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the full dataset outputs for this indicator.
///
/// Performs common validation, determines the start index, prepares output vectors,
/// and does a single-pass loop to calculate the indicator values.
/// Returns an Output struct containing the main indicator outputs and any optional outputs.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let periods = (
        options[0] as usize,
        options[1] as usize,
        options[2] as usize,
    );
    
    validate_inputs(inputs, min_data(options))?;
    let [high, low, close] = inputs;

    let mut ultosc_line = {
        let capacity = output_length(high.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = State::init_state(high, low, close, periods, &mut ultosc_line);
    // Single-pass calculation loop.
    cycle(
        &high[periods.2 + 1..],
        &low[periods.2 + 1..],
        &close[periods.2 + 1..],
        (periods.0, periods.1),
        &mut state,
        &mut ultosc_line[1..],
    );

    Ok((
        vec![ultosc_line],
        IndicatorState {
            periods: (periods.0, periods.1),
            state,
        },
    ))
}

/// Calculates the indicator outputs from a previous state.
///
/// This function is used when only a subset (usually the most recent bars) need to be recalculated.
/// It validates the previous state and merges it with new input data if necessary.
/// For now, this template simply calls the full calculation.

/// A common cycle loop through the data.
/// This template example simply uses the calc() function to produce a result vector.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    periods: (usize, usize),
    state: &mut State,
    ultosc_line: &mut [f64],
) {
    for i in 0..high.len() {
        unsafe {
            *ultosc_line.get_unchecked_mut(i) = state.calc_unchecked(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                periods,
            );
        }
    }
}
