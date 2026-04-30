use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::vwma_simd::indicator_by_options as indicator;
}

pub fn info() -> Info<'static> {
    Info {
        name: "vwma",
        full_name: "Volume Weighted Moving Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        // Two inputs: close and volume.
        inputs: &["close", "volume"],
        // One option: period.
        options: &["period"],
        outputs: &["vwma"],
        // No optional outputs.
        optional_outputs: &[],
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    close: Vec<f64>,
    volume: Vec<f64>,
    state: State,
    period: usize,
}
impl IndicatorState {
    pub fn new(close: &[f64], volume: &[f64], state: State, period: usize) -> Self {
        Self {
            close: close[close.len() - period..].to_vec(),
            volume: volume[volume.len() - period..].to_vec(),
            state,
            period,
        }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.close.extend_from_slice(inputs[0]);
        self.volume.extend_from_slice(inputs[1]);

        let mut vwma_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            &self.close,
            &self.volume,
            self.period,
            &mut self.state,
            &mut vwma_line,
        );

        self.close.drain(..self.close.len() - self.period);
        self.volume.drain(..self.volume.len() - self.period);

        Ok(vec![vwma_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub sum: f64,
    pub vol_sum: f64,
}
impl State {
    pub fn new(sum: f64, vol_sum: f64) -> Self {
        State { sum, vol_sum }
    }
    /// Initializes VWMA by computing the initial numerator and denominator sums over the first period,
    /// then computing the first VWMA value.
    pub fn init_state(period: usize, close: &[f64], volume: &[f64]) -> Self {
        let mut sum = 0.0;
        let mut vol_sum = 0.0;
        for i in 0..period {
            sum += close[i] * volume[i];
            vol_sum += volume[i];
        }
        Self::new(sum, vol_sum)
    }
    #[inline(always)]
    pub fn calc(&mut self, values: (&f64, &f64), prev_values: (&f64, &f64)) -> f64 {
        let (close, volume) = values;
        let (prev_close, prev_volume) = prev_values;
        // Add new bar's contribution.
        self.sum += (close * volume) - (prev_close * prev_volume);
        self.vol_sum += volume - prev_volume;

        if self.vol_sum == 0.0 {
            return 0.0;
        }
        self.sum / self.vol_sum
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum required data points equal to the period.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Returns the output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Full-indicator calculation for VWMA.
/// Calculates the Volume Weighted Moving Average (VWMA) indicator for the entire dataset.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let close = inputs[0];
    let volume = inputs[1];

    let mut vwma_line = {
        let capacity = output_length(close.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    // Initialize state.
    let mut state = State::init_state(period, close, volume);

    // Process from index = period (first full window is available).
    cycle(close, volume, period, &mut state, &mut vwma_line);

    Ok((
        vec![vwma_line],
        IndicatorState::new(close, volume, state, period),
    ))
}

/// Cycle through the close and volume arrays starting at `period` and updates the VWMA values.
fn cycle(close: &[f64], volume: &[f64], period: usize, state: &mut State, vwma_line: &mut [f64]) {
    for (j, i) in (period..close.len()).enumerate() {
        unsafe {
            *vwma_line.get_unchecked_mut(j) = state.calc(
                (close.get_unchecked(i), volume.get_unchecked(i)),
                (
                    close.get_unchecked(j),
                    volume.get_unchecked(j),
                ),
            )
        };
    }
}

/// Per-bar calculation for VWMA.
/// Updates the numerator and denominator for a sliding window of `period` elements.
/// For index i, the oldest data point is at i - period.
/// Returns (vwma, new_sum, new_vol_sum).
#[inline(always)]
pub fn calc(state: &mut State, values: (&f64, &f64), prev_values: (&f64, &f64)) -> f64 {
    state.calc(values, prev_values)
}
