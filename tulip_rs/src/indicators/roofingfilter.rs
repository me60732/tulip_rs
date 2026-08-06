//! # Ehlers Roofing Filter
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 2.
//!
//! A band-pass pre-filter that cascades the High Pass and Super Smoother filters
//! to band-limit a price signal to the cycle frequencies of interest. The name
//! comes from the idea that the combined filter creates a "roof" at the high end
//! (via the Super Smoother) and a "floor" at the low end (via the High Pass),
//! confining the output to a specific frequency band.
//!
//! ## Pipeline
//!
//! ```text
//! Price
//!   │
//!   ▼
//! High Pass filter  (cutoff = hp_period bars)   removes DC trend / long-cycle drift
//!   │
//!   ▼
//! Super Smoother    (cutoff = ss_period bars)   removes high-frequency noise / aliasing
//!   │
//!   ▼
//! Roofed signal  (band-limited to [ss_period, hp_period] bar cycles)
//! ```
//!
//! Options: `[ss_period, hp_period]`.  A typical configuration is
//! `ss_period = 10, hp_period = 48`, preserving 10–48 bar cycles.
//!
//! ## Role in this library
//!
//! Used as the first stage of [`hilberttransform`], which applies the
//! Hilbert kernel to the roofed (band-limited) signal rather than to raw price.
//! This is the key architectural difference between our Hilbert Transform
//! (Ehlers 2013) and TA-Lib's `HT_PHASOR` (Ehlers 2001), which applies the
//! kernel directly to a simple WMA-smoothed price.

use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::{
    highpass::{HighPass, State as HpState},
    supersmoother::State as SsState,
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};

use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::roofingfilter_simd::indicator_by_options as indicator;
}


pub type IndicatorState = State;
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut rf_line, mut hp_line) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    hp_line: len
                ),
            )
        };

        cycle(
            inputs[0],
            self,
            (&mut rf_line, &mut hp_line),
        );

        Ok(vec![rf_line, hp_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub hp_state: HpState,
    pub ss_state: SsState, // SuperSmoother (low-pass) state
}
impl State {
    pub fn new((ss_period, hp_period): (usize, usize)) -> Self {
        Self {
            ss_state: SsState::new(ss_period),
            hp_state: HpState::new(hp_period),
        }
    }
    pub fn init_state(
        real: &[f64],
        (ss_period, hp_period): (usize, usize),
        hp_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new((ss_period, hp_period));
        let l_period = ss_period.max(hp_period);
        for (i, &value) in real.iter().take(l_period).enumerate() {
            let (_, hp) = state.calc(value);
            crate::init_store_optional_outputs!(i, real.len(),
                hp_line => hp
            );
        }
        state
    }
    
}
impl TState for State {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let hp = self.hp_state.calc(real);
        (self.ss_state.calc(hp), hp)
    }
}


/// Performs the core filter loop for the RoofingFilter indicator.
///
/// # Arguments
///
/// * `real` - A slice of input price values.
/// * `state` - A mutable reference to the composite filter state (`ss_state`, `hp_state`).
/// * `multipliers` - The precomputed filter coefficients `((a1, a2, b0), (a1, a2))`.
/// * `outputs` - Tuple of `(rf_line, hp_line)` output slices; `rf_line` must be the same length as `real`.
fn cycle(
    real: &[f64],
    state: &mut State,
    (rf_line, hp_line): (&mut [f64], &mut [f64]),
) {
    let (_, want_hp) = crate::calc_want_flags!(hp_line);
    for i in 0..real.len() {
        let (rf, hp);
        unsafe {
            (rf, hp) = state.calc(*real.get_unchecked(i));
            *rf_line.get_unchecked_mut(i) = rf;
        }
        crate::store_optional_outputs!(i,
            want_hp, hp_line => hp
        );
    }
}


pub struct RoofingFilter;
impl Indicator<INPUTS, OPTIONS> for RoofingFilter {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "roofingfilter",
        indicator_type: IndicatorType::Math,
        full_name: "Ehlers Roofing Filter",
        inputs: &["real"],
        options: &["ss_period", "hp_period"],
        outputs: &["roofing"],
        optional_outputs: &["highpass"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "roofing",
            label: "Ehlers Roofing Filter",
            display_type: DisplayType::Indicator,
            outputs: &["roofing", "highpass"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0].max(options[1]) as usize + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let periods = (options[0] as usize, options[1] as usize);

        validate_inputs(inputs, Self::min_data(options))?;
    
        let (mut rf_line, mut hp_line) = {
            let capacity = Self::output_length(inputs[0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    hp_line: HighPass::output_length(inputs[0].len(), &[periods.1 as f64])
                ),
            )
        };
        let mut state = State::init_state(inputs[0], periods, &mut hp_line);
    
        let real = &inputs[0][periods.0.max(periods.1)..];
        let outputs = {
            let offset = crate::slice_outputs_start!(rf_line.len(), hp_line);
            (rf_line.as_mut_slice(), &mut hp_line[offset..])
        };
        cycle(real, &mut state, outputs);
    
        Ok((
            vec![rf_line, hp_line],
            state,
        ))
    }
}