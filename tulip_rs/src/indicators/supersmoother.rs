//! # Ehlers Super Smoother
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 2.
//! Also published as "Predictive Indicators for Effective Trading Strategies",
//! *Technical Analysis of Stocks & Commodities*, January 2014.
//!
//! A two-pole Butterworth-inspired IIR low-pass filter designed to remove
//! aliasing and high-frequency noise from sampled price data while preserving
//! cycle content below the cutoff frequency. Unlike a simple moving average it
//! has zero lag at DC and a much sharper roll-off, making it Ehlers' preferred
//! smoothing primitive for cycle analysis.
//!
//! ## Formula
//!
//! Given `ω = π / period` (note: π, not 2π — a half-cycle convention):
//!
//! ```text
//! a1 = 2 · exp(−√2 · ω) · cos(√2 · ω)      [Ehlers uses 1.414 for √2]
//! a2 = −exp(−2√2 · ω)
//! b0 = 1 − a1 − a2
//! SS = (b0 / 2) · (Price + Price[1]) + a1·SS[1] + a2·SS[2]
//! ```
//!
//! The `b0/2` feedforward ensures unit gain at DC so the smoother tracks
//! the mean of price without bias.
//!
//! ## Role in this library
//!
//! Used as the second stage of the [`roofingfilter`] (after the High Pass
//! filter) and transitively in [`hilberttransform`]. On its own it acts as a
//! high-quality low-pass filter for any price series.

use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State;
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut super_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs[0], self, &mut super_line);

        Ok(vec![super_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    // previous outputs
    pub y1: f64,        // y[t-1]
    pub y2: f64,        // y[t-2]
    pub prev_real: f64, // x[t-1] for Ehlers input averaging: (Close + Close[1]) / 2
    pub a1: f64,
    pub a2: f64,
    pub b0: f64,
}
impl State {
    pub fn new(period: usize) -> Self {
        let (a1, a2, b0) = multiplier(period);
        Self {
            y1: 0.0,
            y2: 0.0,
            prev_real: 0.0,
            a1,
            a2,
            b0,
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
impl TState for State {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        // Ehlers: coeff/2 * (Close + Close[1]) + a1*y1 + a2*y2
        let y = self.b0.mul_add(
            real + self.prev_real,
            self.a1.mul_add(self.y1, self.a2 * self.y2),
        );
        self.y2 = self.y1;
        self.y1 = y;
        self.prev_real = real;
        y
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
fn cycle(real: &[f64], state: &mut State, super_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *super_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
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
pub fn multiplier(period: usize) -> (f64, f64, f64) {
    let omega = std::f64::consts::PI / period as f64;

    let a1 = 2.0 * (-1.414 * omega).exp() * (1.414 * omega).cos();
    let a2 = -(-2.828 * omega).exp();
    let b0 = (1.0 - a1 - a2) * 0.5;

    (a1, a2, b0)
}

pub struct SuperSmoother;

impl Indicator<INPUTS, OPTIONS> for SuperSmoother {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "supersmoother",
        indicator_type: IndicatorType::Math,
        full_name: "Ehlers Super Smoother",
        inputs: &["real"],
        options: &["period"],
        outputs: &["supersmoother"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "supersmoother",
            label: "Ehlers Super Smoother",
            display_type: DisplayType::Overlay,
            outputs: &["supersmoother"],
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

        let mut super_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::init_state(inputs[0], period);

        let real = &inputs[0][period..];
        cycle(real, &mut state, &mut super_line);

        Ok((vec![super_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::supersmoother_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for SuperSmoother {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::supersmoother_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
