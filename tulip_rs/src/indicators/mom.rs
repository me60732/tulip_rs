use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::mom_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::mom_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::mom_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::mom_simd::indicator_by_options as indicator;
}

/// Returns information about the Momentum (MOM) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "mom",
        full_name: "Momentum",
        indicator_type: IndicatorType::Momentum,
        display_type: DisplayType::Indicator,
        inputs: &["real"],
        options: &["period"],
        outputs: &["mom"],
        optional_outputs: &[],
    }
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
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut mom_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_mom(&self.real, self.period, &mut mom_line);

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![mom_line])
    }
}

pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the MOM indicator.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the SMA calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;
    
    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let mut mom_line = {
        let capacity = output_length(real.len(), options);
        crate::uninit_vec!(f64, capacity)
    };

    cycle_mom(real, period, &mut mom_line);

    Ok((vec![mom_line], IndicatorState::new(real, period)))
}

/// Iterates over the input data and applies the calc function.
fn cycle_mom(real: &[f64], period: usize, mom_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        unsafe {
            *mom_line.get_unchecked_mut(j) =
                calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Performs the core calculation for the Momentum (MOM) indicator.
#[inline(always)]
pub fn calc(real: f64, prev_real: f64) -> f64 {
    real - prev_real
}
