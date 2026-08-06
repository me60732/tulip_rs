use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::sma::{Sma, State as SmaState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dpo_simd::indicator_by_options as indicator;
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
#[repr(transparent)]
pub struct State<S = Cold>(pub SmaState<S>);
impl<S> Deref for State<S> {
    type Target = SmaState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl State<Cold> {
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        State(SmaState::init_state(real, period))
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value, dpo_price): Self::Inputs<'a>) -> Self::Outputs {
        let sma = self.0.calc((value, prev_value));
        (dpo_price - sma, sma)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    real: Vec<f64>,
    dpo_period: usize,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize, dpo_period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
            dpo_period,
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

        let (mut dpo_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_dpo(
            &self.real,
            (self.period, self.dpo_period),
            &mut self.state,
            &mut dpo_line,
            &mut sma_line,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![dpo_line, sma_line])
    }
}

/// Performs the main calculation loop for the DPO indicator.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `periods` - A tuple `(period, dpo_period)` where `period` is the SMA window and
///   `dpo_period` is the look-back offset used to detrend the price.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
/// * `sum` - Mutable reference to the running sum used for the SMA calculation.
/// * `dpo_line` - Mutable slice to write the DPO output values into.
/// * `sma_line` - Mutable slice to write the SMA values into (optional output).
fn cycle_dpo(
    real: &[f64],
    periods: (usize, usize),
    state: &mut State<Warm>,
    dpo_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (period, dpo_period) = periods;
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let inputs = unsafe {
            (
                *real.get_unchecked(i),
                *real.get_unchecked(j),
                *real.get_unchecked(i - dpo_period),
            )
        };
        let (dpo, sma) = state.calc(inputs);
        unsafe {
            *dpo_line.get_unchecked_mut(j) = dpo;
        }
        crate::store_optional_outputs!(j, want_sma, sma_line => sma);
    }
}

pub struct Dpo;

impl Indicator<INPUTS, OPTIONS> for Dpo {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "dpo",
        indicator_type: IndicatorType::Cycle,
        full_name: "Detrended Price Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["dpo"],
        optional_outputs: &["sma"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "dpo",
                label: "DPO",
                display_type: DisplayType::Indicator,
                outputs: &["dpo"],
            },
            DisplayGroup {
                offset: None,
                id: "sma",
                label: "SMA",
                display_type: DisplayType::Overlay,
                outputs: &["sma"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;
        let dpo_period = period / 2 + 1;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let (mut dpo_line, mut sma_line) = {
            let sma_capacity = Sma::output_length(real.len(), options);
            let capacity = Self::output_length(real.len(), options);

            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: sma_capacity
                ),
            )
        };

        let mut state = State::init_state(real, period);
        // Perform the main DPO calculation
        cycle_dpo(
            real,
            (period, dpo_period),
            &mut state,
            &mut dpo_line,
            &mut sma_line,
        );

        Ok((
            vec![dpo_line, sma_line],
            IndicatorState::new(real, state, period, dpo_period),
        ))
    }
}
