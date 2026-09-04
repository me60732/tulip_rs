use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

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

        let mut mom_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_mom(&self.real, self.period, &mut mom_line);

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![mom_line])
    }
}

/// Iterates over the input data and applies the calc function.
fn cycle_mom(real: &[f64], period: usize, mom_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        unsafe {
            *mom_line.get_unchecked_mut(j) = calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Performs the core calculation for the Momentum (MOM) indicator.
#[inline(always)]
pub fn calc(real: f64, prev_real: f64) -> f64 {
    real - prev_real
}

pub struct Mom;
impl Indicator<INPUTS, OPTIONS> for Mom {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "mom",
        full_name: "Momentum",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["mom"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "mom",
            label: "MOM",
            display_type: DisplayType::Indicator,
            outputs: &["mom"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut mom_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        cycle_mom(real, period, &mut mom_line);

        Ok((vec![mom_line], IndicatorState::new(real, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mom_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Mom {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mom_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
