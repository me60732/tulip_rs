use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, TState, Indicator, IndicatorResult};
//use crate::indicators::stddev::Calc as StdDevCalc;
use crate::indicators::stddev::State as StddevState;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::bbands_simd::indicator_by_options as indicator;
}


#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub stddev_state: StddevState<S>,
    pub std_dev: f64
}
impl State {
    pub fn init_state(real: &[f64], period: usize, std_dev: f64) -> State<Warm> {
        let stddev_state = StddevState::init_state(real, period);
        State {
            std_dev,
            stddev_state
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
        let (sd, sma);
        (sd, sma) = self.stddev_state.calc(inputs);
    
        let upper_band = self.std_dev.mul_add(sd, sma);
        let lower_band = (-self.std_dev).mul_add(sd, sma);
    
        (lower_band, sma, upper_band)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
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
        let period = self.period;

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

        cycle_bbands(
            &self.real,
            period,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut self.state,
        );

        self.real.drain(..self.real.len() - period);

        Ok(vec![lower_band, middle_band, upper_band])
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}


/// Performs the main calculation loop for the BBANDS indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `period` - The period for the BBANDS calculation.
/// * `std_dev` - The standard deviation multiplier for the bands.
/// * `multiplier` - The precomputed period multiplier used in standard deviation calculation.
/// * `outputs` - A tuple of mutable slices for storing the lower, middle, and upper bands.
/// * `state` - A mutable reference to the current indicator state.
fn cycle_bbands(
    real: &[f64],
    period: usize,
    (lower_band, middle_band, upper_band): (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State<Warm>,
) {
    for (j, i) in (period..real.len()).enumerate() {

        let (lower, middle, upper) = state.calc((
            unsafe { *real.get_unchecked(i) },
            unsafe { *real.get_unchecked(j) },
        ));
        unsafe {
            *middle_band.get_unchecked_mut(j) = middle;
            *upper_band.get_unchecked_mut(j) = upper;
            *lower_band.get_unchecked_mut(j) = lower;
        }
    }
}


pub struct BBands;

impl Indicator<INPUTS, OPTIONS> for BBands {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "bbands",
        full_name: "Bollinger Bands",
        indicator_type: IndicatorType::Volatility,
        inputs: &["real"],
        options: &["period", "std_dev"],
        outputs: &["bbands_lower", "bbands_middle", "bbands_upper"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "bbands",
            label: "BBANDS",
            display_type: DisplayType::Overlay,
            outputs: &["bbands_lower", "bbands_middle", "bbands_upper"],
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
        let std_dev = options[1];
    
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
    
        let mut state = State::init_state(real, period, std_dev);
        cycle_bbands(
            real,
            period,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut state,
        );
    
        Ok((
            vec![lower_band, middle_band, upper_band],
            IndicatorState::new(real, state, period),
        ))
    }
    
}