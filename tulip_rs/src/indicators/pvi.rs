use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 2;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::pvi_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::pvi_simd::indicator_by_assets as indicator;
}

/// Returns information about the Negative Volume Index (pvi) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "pvi",
        full_name: "Positive Volume Index",
        indicator_type: IndicatorType::Volume,
        display_type: DisplayType::Indicator,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["pvi"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub pvi: f64,
    pub close: f64,
    pub volume: f64,
}
impl IndicatorState {
    #[inline(always)]
    pub fn new(pvi: f64, close: f64, volume: f64) -> Self {
        Self { pvi, close, volume }
    }
    #[inline(always)]
    pub fn calc(&mut self, close: f64, volume: f64) -> f64 {
        if volume > self.volume {
            //return pvi + (close - prev_close) / prev_close * pvi
            self.pvi = close / self.close * self.pvi;
        }
        (self.close, self.volume) = (close, volume);
        self.pvi
    }
    fn cycle(&mut self, close: &[f64], volume: &[f64], pvi_line: &mut [f64]) {
        for i in 0..close.len() {
            unsafe {
                *pvi_line.get_unchecked_mut(i) =
                    self.calc(*close.get_unchecked(i), *volume.get_unchecked(i));
            }
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
        let close = inputs[0];
        let volume = inputs[1];

        let mut pvi_line = crate::uninit_vec!(f64, close.len());

        self.cycle(&close, &volume, &mut pvi_line);

        Ok(vec![pvi_line])
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the pvi indicator.
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Returns the output length for the pvi indicator.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len - min_data(_options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let close = inputs[0];
    let volume = inputs[1];
    let mut pvi_line = {
        let capacity = output_length(close.len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle(close, volume, &mut pvi_line, 1000.0);
    let pvi = pvi_line[pvi_line.len() - 1];
    Ok((
        vec![pvi_line],
        IndicatorState {
            pvi,
            close: close[close.len() - 1],
            volume: volume[volume.len() - 1],
        },
    ))
}

/// Iterates over the input data and applies the calc function.
fn cycle(close: &[f64], volume: &[f64], pvi_line: &mut [f64], mut pvi: f64) {
    for (j, i) in (1..close.len()).enumerate() {
        unsafe {
            pvi = calc(
                close.get_unchecked(i),
                close.get_unchecked(j),
                volume.get_unchecked(i),
                volume.get_unchecked(j),
                pvi,
            );
            *pvi_line.get_unchecked_mut(j) = pvi;
        }
    }
}

/// Performs the core calculation for the Negative Volume Index (pvi) indicator.
#[inline(always)]
pub fn calc(close: &f64, prev_close: &f64, volume: &f64, prev_volume: &f64, pvi: f64) -> f64 {
    if volume > prev_volume {
        //return pvi + (close - prev_close) / prev_close * pvi
        return close / prev_close * pvi;
    }

    pvi
}
