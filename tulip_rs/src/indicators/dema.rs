use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
pub use crate::indicators::ema::multiplier;
use crate::indicators::ema::{calc as calc_ema, Ema, State as EmaState};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dema_simd::indicator_by_options as indicator;
}

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub ema_state: EmaState<S>,
    pub ema2: f64,
}
impl<S> Deref for State<S> {
    type Target = EmaState<S>;
    fn deref(&self) -> &Self::Target { &self.ema_state }
}
impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ema_state }
}
impl State<Cold> {
    #[allow(unused)]
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            ema_state: self.ema_state.into_warm(),
            ema2: self.ema2,
        }
    }
    pub fn init_state(real: &[f64], period: usize, ema_line: &mut [f64]) -> State<Warm> {
        let init_bars = period * 2 - 3;

        let ema_state = EmaState::init_state(real, period);

        // Seed ema2 once, at the transition point
        let mut state = State::<Warm> {
            ema2: ema_state.ema,
            ema_state
        };

        // Phase 2: run full DEMA calc for the remaining init bars
        for i in period..=init_bars {
            let (_, ema) = state.calc(real[i]);
            crate::init_store_optional_outputs!(i, real.len(), ema_line => ema);
        }

        state
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        let ema1 = self.ema_state.calc(value);
        self.ema2 = calc_ema(ema1, self.ema2, self.multiplier, self.inv_multiplier);
        //(2.0 * state.ema1 - state.ema2, state.ema1)
        (ema1.mul_add(2.0, -self.ema2), ema1)
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut dema_line, mut ema_line) = {
            let capacity = inputs[0].len();
            //let mut dema_line = vec![0.0; capacity];
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                ),
            )
        };
        cycle_dema(
            inputs[0],
            self,
            &mut dema_line,
            &mut ema_line,
        );

        Ok(vec![dema_line, ema_line])
    }
}


/// Performs the main calculation loop for the DEMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `multipliers` - A tuple of EMA multipliers derived from the period.
/// * `state` - Mutable reference to the DEMA state holding `ema1` and `ema2`.
/// * `dema_line` - Mutable slice to write the DEMA output values into.
/// * `ema_line` - Mutable slice to write the EMA output values into (optional output).
fn cycle_dema(
    real: &[f64],
    state: &mut State<Warm>,
    dema_line: &mut [f64],
    ema_line: &mut [f64],
) {
    let (_, want_ema) = crate::calc_want_flags!(ema_line);

    for i in 0..real.len() {
        let value = unsafe { *real.get_unchecked(i) };

        let (dema, ema) = state.calc(value);

        unsafe { *dema_line.get_unchecked_mut(i) = dema };

        crate::store_optional_outputs!(i,
            want_ema, ema_line => ema
        );
    }
}

pub struct Dema;
impl Indicator<INPUTS, OPTIONS> for Dema {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "dema",
        indicator_type: IndicatorType::Trend,
        full_name: "Double Exponential Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["dema"],
        optional_outputs: &["ema"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "dema_ema",
            label: "EMA",
            display_type: DisplayType::Overlay,
            outputs: &["dema", "ema"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize * 2 - 1
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        validate_inputs(inputs, Self::min_data(options))?;
    
        let (mut dema_line, mut ema_line, mut state);
        {
            let capacity = Self::output_length(inputs[0].len(), options);
            let ema_capacity = Ema::output_length(inputs[0].len(), options);
    
            dema_line = crate::uninit_vec!(f64, capacity);
    
            // Initialize any optional outputs
            ema_line = crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                ema_line: ema_capacity
            );
    
            state = State::init_state(inputs[0], /*capacity, */period, &mut ema_line);
        }
        let ema = {
            let offset = crate::slice_outputs_start!(dema_line.len(), ema_line);
            &mut ema_line[offset..]
        };
    
        cycle_dema(
            &inputs[0][period * 2 - 2..],
            &mut state,
            &mut dema_line,
            ema,
        );
    
        Ok((
            vec![dema_line, ema_line],
            state,
        ))
    }
}