use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::sma::State as SmaState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub sma_state: SmaState<S>,
    pub percentage: f64,
}
impl State<Cold> {
    pub fn new(sum: f64, period: usize, percentage: f64) -> Self {
        let percentage = percentage / 100.0;
        Self {
            percentage,
            sma_state: SmaState::new(sum, period),
        }
    }
    pub fn init_state(real: &[f64], period: usize, percentage: f64) -> State<Warm> {
        let sma_state = SmaState::init_state(real, period);
        let percentage = percentage / 100.0;
        State {
            sma_state,
            percentage,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let sma = self.sma_state.calc(inputs);
        let step = sma * self.percentage;

        (sma - step, sma, sma + step)
    }
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
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

        let (mut middle_band, mut upper_band, mut lower_band) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        cycle(
            &self.real,
            self.period,
            &mut self.state,
            (&mut lower_band, &mut middle_band, &mut upper_band),
        );

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![lower_band, middle_band, upper_band])
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Performs the main calculation loop for the SMA Envelope indicator.
///
/// Iterates over `real[period..]`, advancing the rolling sum one bar at a time
/// and writing the lower, middle, and upper band values into `outputs`.
///
/// # Arguments
///
/// * `real` - A slice of real (price) values.
/// * `period` - The SMA look-back period.
/// * `multipliers` - Precomputed `(1/period, percentage/100)` tuple.
/// * `outputs` - A tuple of mutable slices `(lower, middle, upper)` to write into.
/// * `sum` - The running sum of the current SMA window (updated in place).
//#[inline(always)]
fn cycle(
    real: &[f64],
    period: usize,
    state: &mut State<Warm>,
    (lower_band, middle_band, upper_band): (&mut [f64], &mut [f64], &mut [f64]),
) {
    for (j, i) in (period..real.len()).enumerate() {
        let inputs = unsafe { (*real.get_unchecked(i), *real.get_unchecked(i - period)) };

        let (lower, middle, upper) = state.calc(inputs);
        unsafe {
            *middle_band.get_unchecked_mut(j) = middle;
            *upper_band.get_unchecked_mut(j) = upper;
            *lower_band.get_unchecked_mut(j) = lower;
        }
    }
}

pub struct SmaEnvelope;

impl Indicator<INPUTS, OPTIONS> for SmaEnvelope {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "smaenvelope",
        full_name: "SMA Envelope",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period", "percentage"],
        outputs: &["lower", "middle", "upper"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "smaenvelope",
            label: "SMA Envelope",
            display_type: DisplayType::Overlay,
            outputs: &["lower", "middle", "upper"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let percentage = options[1];

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let (mut middle_band, mut upper_band, mut lower_band) = {
            let capacity = Self::output_length(real.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let mut state = State::init_state(real, period, percentage);
        cycle(
            real,
            period,
            &mut state,
            (&mut lower_band, &mut middle_band, &mut upper_band),
        );

        Ok((
            vec![lower_band, middle_band, upper_band],
            IndicatorState::new(real, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for SmaEnvelope {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
