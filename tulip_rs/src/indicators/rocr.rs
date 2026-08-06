use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::rocr_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::rocr_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::rocr_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::rocr_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], period: usize) -> Self {
        Self {
            period,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut rocr_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_rocr(&self.real, self.period, &mut rocr_line);

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![rocr_line])
    }
}

/// Iterates over the input data and applies the calc function.
fn cycle_rocr(real: &[f64], period: usize, rocr_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        unsafe {
            *rocr_line.get_unchecked_mut(j) = calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Performs the core calculation for the Rate of Change Ratio (ROCR) indicator.
#[inline(always)]
pub fn calc(real: f64, prev_real: f64) -> f64 {
    real / prev_real.max(f64::EPSILON)
}

pub struct Rocr;

impl Indicator<INPUTS, OPTIONS> for Rocr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "rocr",
        full_name: "Rate of Change Ratio",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["rocr"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "rocr",
            label: "ROCR",
            display_type: DisplayType::Indicator,
            outputs: &["rocr"],
        }],
    };

    /// Returns the minimum amount of data required for the ROCR indicator.
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }
    /// Calculates the output length for the ROCR indicator given the input data length and options.
    ///
    /// # Arguments
    ///
    /// * `data_len` - The length of the input data.
    /// * `options` - A slice containing the options for the ROCR calculation.
    ///
    /// # Returns
    ///
    /// The output length.
    fn output_length(data_len: usize, options: &[f64; OPTIONS]) -> usize {
        data_len - Self::min_data(options)
    }
    
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        if options[0] < 1.0 {
            return Err(IndicatorError::InvalidOptions);
        }
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
    
        let mut rocr_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };
    
        cycle_rocr(real, period, &mut rocr_line);
    
        Ok((vec![rocr_line], IndicatorState::new(real, period)))
    }
}
