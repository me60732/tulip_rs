use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::mom::calc as calc_mom;
use crate::indicators::rocr::calc as calc_rocr;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::roc_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::roc_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::roc_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::roc_simd::indicator_by_options as indicator;
}

/// Returns information about the Rate of Change (ROC) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "roc",
        full_name: "Rate of Change",
        indicator_type: IndicatorType::Momentum,
        display_type: DisplayType::Indicator,
        inputs: &["real"],
        options: &["period"],
        outputs: &["roc"],
        optional_outputs: &["mom"],
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
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let (mut roc_line, mut mom_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    mom_line: capacity
                ),
            )
        };

        cycle_roc(&self.real, self.period, &mut roc_line, &mut mom_line);
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![roc_line, mom_line])
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the ROC indicator.
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
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];

    let (mut roc_line, mut mom_line) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                mom_line: capacity
            ),
        )
    };

    cycle_roc(real, period, &mut roc_line, &mut mom_line);

    Ok((
        vec![roc_line, mom_line],
        IndicatorState {
            period,
            real: real[real.len() - period..].to_vec(),
        },
    ))
}

/// Iterates over the input data and applies the calc function.
fn cycle_roc(real: &[f64], period: usize, roc_line: &mut [f64], mom_line: &mut [f64]) {
    let (_, want_mom) = crate::calc_want_flags!(mom_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (roc, mom) = unsafe { calc(*real.get_unchecked(i), *real.get_unchecked(j)) };
        unsafe { *roc_line.get_unchecked_mut(j) = roc };
        crate::store_optional_outputs_safe!(j,
            want_mom, mom_line => mom
        );
    }
}

/// Performs the core calculation for the Rate of Change (ROC) indicator.
#[inline(always)]
pub fn calc(real: f64, prev_real: f64) -> (f64, f64) {
    let mom = calc_mom(real, prev_real);
    (calc_rocr(mom, prev_real), mom)
}
