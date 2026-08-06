//! # Ehlers High Pass Filter
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 2.
//! Also described in *Cybernetic Analysis for Stocks and Futures* (2004).
//!
//! A one-pole IIR high-pass filter that removes the trend (DC component and
//! sub-cycle drift) from price, leaving only the oscillatory cycle content for
//! downstream analysis. The single `period` option sets the cutoff: cycles
//! longer than `period` bars are suppressed; shorter cycles pass through.
//!
//! ## Formula
//!
//! Given `ω = 2π / period`:
//!
//! ```text
//! α  = (1 − sin ω) / cos ω
//! b  = (1 + α) / 2
//! HP = b·(Price − Price[1]) + α·HP[1]
//! ```
//!
//! Internally the coefficients are stored as `(a1, a2) = (α, b)` and evaluated as
//! `HP = a1·HP[1] + a2·(Price − Price[1])`.
//!
//! ## Role in this library
//!
//! Used as the first stage of the [`roofingfilter`] and (transitively) the
//! [`hilberttransform`] indicator. On its own it outputs the de-trended signal,
//! which is not directly tradeable but is essential for cycle-period estimation.

use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, TState, Indicator, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::highpass_simd::indicator_by_options as indicator;
}


pub type IndicatorState = State;
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut highpass_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            self,
            &mut highpass_line,
        );

        Ok(vec![highpass_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub y1: f64, // y[t-1]
    pub prev_real: f64,
    pub a1: f64,
    pub a2: f64
}
impl TState for State {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let y = self.a1.mul_add(self.y1, self.a2 * (real - self.prev_real));
        self.prev_real = real;
        self.y1 = y;
        y
    }
}
impl State {
    pub fn new(period: usize) -> Self {
        let (a1, a2) = multiplier(period);
        Self {
            y1: 0.0,
            prev_real: 0.0,
            a1,
            a2,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> Self {
        let mut state = Self::new(period);
        for &value in real.iter().take(period) {
            state.calc(value);
        }
        state
    }
    
}


/// Performs the core filter loop for the SuperSmoother indicator.
///
/// # Arguments
///
/// * `real` - A slice of input price values.
/// * `state` - A mutable reference to the filter state (`y1`, `y2`).
/// * `multipliers` - The precomputed filter coefficients `(a1, a2, b0)`.
/// * `super_line` - Output slice for the filtered values (must be the same length as `real`).
fn cycle(real: &[f64], state: &mut State, highpass_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *highpass_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
        }
    }
}

/// Computes the 2-pole SuperSmoother filter coefficients for a given period.
///
/// # Arguments
///
/// * `period` - The filter period. Controls the cutoff frequency of the smoother.
///
/// # Returns
///
/// A tuple `(a1, a2, b0)` where:
/// - `a1`, `a2` are the IIR feedback coefficients
/// - `b0` is the feedforward gain (`1 - a1 - a2`)
pub fn multiplier(period: usize) -> (f64, f64) {
    let omega = std::f64::consts::TAU / period as f64;

    let a1 = (1.0 - omega.sin()) / omega.cos();
    let a2 = 0.5 * (1.0 + a1);

    (a1, a2)
}


pub struct HighPass;
impl Indicator<INPUTS, OPTIONS> for HighPass {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "highpass",
        indicator_type: IndicatorType::Math,
        full_name: "Ehlers High Pass Filter",
        inputs: &["real"],
        options: &["period"],
        outputs: &["highpass"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "highpass",
            label: "Ehlers High Pass",
            display_type: DisplayType::Indicator,
            outputs: &["highpass"],
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
    
        let mut highpass_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::init_state(inputs[0], period);
    
        let real = &inputs[0][period..];
        cycle(real, &mut state, &mut highpass_line);
    
        Ok((vec![highpass_line], state))
    }
}