use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::wad_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::wad_simd::indicator_by_assets as indicator;
}

pub fn info() -> Info<'static> {
    Info {
        name: "wad",
        full_name: "WAD Indicator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["wad"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub prev_close: f64,
    pub wad: f64,
}
impl IndicatorState {
    pub fn new(prev_close: f64, wad: f64) -> Self {
        Self { prev_close, wad }
    }
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64) -> f64 {
        self.wad += if close > self.prev_close { 
            close - self.prev_close.min(low) 
        } else if close < self.prev_close { 
            close - self.prev_close.max(high) } 
        else { 
            0.0 
        };

        self.prev_close = close;

        self.wad
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut wad_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs[0], inputs[1], inputs[2], self, &mut wad_line);

        Ok(vec![wad_line])
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum required data points. We need at least 2 bars:
/// one to initialize the state (previous close) and then a new bar for a calculation.
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Returns the output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Full-indicator calculation for WAD.
/// It computes a cumulative sum using the previous bar's close as state.
/// Output at index 0 is 0.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    // Expecting three inputs: High, Low, Close.
    validate_inputs(inputs, min_data(_options))?;

    let mut wad_line: Vec<f64> = {
        let capacity = output_length(inputs[0].len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = IndicatorState::new(inputs[2][0], 0.0);

    cycle(
        &inputs[0][1..],
        &inputs[1][1..],
        &inputs[2][1..],
        &mut state,
        &mut wad_line,
    );

    // Store last used close and sum for incremental updates.
    Ok((vec![wad_line], state))
}

/// Cycle through the close and volume arrays starting at `start` and update the VWMA values.
/// Uses the sliding window defined by `period` with the current rolling sums passed in.
/// Returns the new rolling sums (sum, vol_sum) after processing all available data.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut IndicatorState,
    wad_line: &mut [f64],
) {
    for i in 0..close.len() {
        unsafe {
            *wad_line.get_unchecked_mut(i) = state.calc(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
        }
    }
}
/// Per-bar calculation that uses the previous bar's close (prev_close)
/// and updates the rolling sum. For bar at index i:
///
/// if close[i] > prev_close: diff = close[i] - min(prev_close, low[i])
/// if close[i] < prev_close: diff = close[i] - max(prev_close, high[i])
/// else: diff = 0
///
/// Then new_sum = prev_sum + diff.
/// Returns (wad, new_prev_close, new_sum)
#[inline(always)]
pub fn calc(high: f64, low: f64, close: f64, state: &mut IndicatorState) -> f64 {
    state.calc(high, low, close)
}
