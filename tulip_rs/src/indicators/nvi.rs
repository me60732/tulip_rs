use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::nvi_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::nvi_simd::indicator_by_assets as indicator;
}

/// Returns information about the Negative Volume Index (NVI) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "nvi",
        full_name: "Negative Volume Index",
        indicator_type: IndicatorType::Volume,
        display_type: DisplayType::Indicator,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["nvi"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub nvi: f64,
    pub close: f64,
    pub volume: f64,
}
impl IndicatorState {
    #[inline(always)]
    pub fn new(nvi: f64, close: f64, volume: f64) -> Self {
        Self { nvi, close, volume }
    }
    #[inline(always)]
    pub fn calc(&mut self, close: f64, volume: f64) -> f64 {
        if volume < self.volume {
            //return nvi + (close - prev_close) / prev_close * nvi
            self.nvi = close / self.close * self.nvi;
        }
        (self.close, self.volume) = (close, volume);
        self.nvi
    }
    fn cycle(&mut self, close: &[f64], volume: &[f64], nvi_line: &mut [f64]) {
        for i in 0..close.len() {
            unsafe {
                *nvi_line.get_unchecked_mut(i) =
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

        let mut nvi_line = crate::uninit_vec!(f64, inputs[0].len());

        self.cycle(inputs[0], inputs[1], &mut nvi_line);

        Ok(vec![nvi_line])
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the NVI indicator.
pub fn min_data(_options: &[f64]) -> usize {
    2
}

/// Returns the output length for the NVI indicator.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len - min_data(_options) + 1
}

/// Calculates the Negative Volume Index (NVI) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close prices
/// * `inputs[1]` — volume
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `_options` - Unused; this indicator takes no options.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `nvi` line and
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let close = inputs[0];
    let volume = inputs[1];

    let mut nvi_line = {
        let capacity = output_length(close.len(), _options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle(close, volume, &mut nvi_line, 1000.0);
    let nvi = nvi_line[nvi_line.len() - 1];
    Ok((
        vec![nvi_line],
        IndicatorState {
            nvi,
            close: close[close.len() - 1],
            volume: volume[volume.len() - 1],
        },
    ))
}

/// Iterates over the input data and applies the calc function.
fn cycle(close: &[f64], volume: &[f64], nvi_line: &mut [f64], mut nvi: f64) {
    for (j, i) in (1..close.len()).enumerate() {
        unsafe {
            nvi = calc(
                close.get_unchecked(i),
                close.get_unchecked(j),
                volume.get_unchecked(i),
                volume.get_unchecked(j),
                nvi,
            );
            *nvi_line.get_unchecked_mut(j) = nvi;
        }
    }
}

/// Performs the core calculation for the Negative Volume Index (NVI) indicator.
#[inline(always)]
pub fn calc(close: &f64, prev_close: &f64, volume: &f64, prev_volume: &f64, nvi: f64) -> f64 {
    if volume < prev_volume {
        //return nvi + (close - prev_close) / prev_close * nvi
        return close / prev_close * nvi;
    }

    nvi
}
