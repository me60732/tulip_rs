use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult};
use crate::indicators::mom::calc as calc_mom;
use crate::indicators::rocr::calc as calc_rocr;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::roc_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::roc_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::roc_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::roc_simd::indicator_by_options as indicator;
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

pub struct Roc;
impl Indicator<INPUTS, OPTIONS> for Roc {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "roc",
        full_name: "Rate of Change",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["roc"],
        optional_outputs: &["mom"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "roc",
                label: "ROC",
                display_type: DisplayType::Indicator,
                outputs: &["roc"],
            },
            DisplayGroup {
                offset: None,
                id: "mom",
                label: "Momentum",
                display_type: DisplayType::Indicator,
                outputs: &["mom"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
    
        let (mut roc_line, mut mom_line) = {
            let capacity = Self::output_length(real.len(), options);
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
}