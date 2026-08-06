use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::sma::State as SmaState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

// SIMD variants are not yet implemented for SMA Envelope.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::smaenvelope_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub sma_state: SmaState<S>,
    pub percentage: f64
}
impl State<Cold> {
    pub fn new(sum: f64, period: usize, percentage: f64) -> Self {
        let percentage = percentage / 100.0;
        Self {
            percentage,
            sma_state: SmaState::new(sum, period)
        }
    }
    pub fn init_state(real: &[f64], period: usize, percentage: f64) -> State<Warm> {
        let sma_state = SmaState::init_state(real, period);
        let percentage = percentage / 100.0;
        State {
            sma_state,
            percentage
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>
    ) -> Self::Outputs {
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
        let inputs =
            unsafe { (*real.get_unchecked(i), *real.get_unchecked(i - period)) };

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
}