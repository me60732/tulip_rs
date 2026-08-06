use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
pub use crate::indicators::stddev::multiplier;
use crate::indicators::stddev::State as StddevState;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::volatility_simd::indicator_by_options as indicator;
}
const ANNUAL: f64 = 15.874507866387544; // 252_f64.sqrt()

pub type IndicatorState = State<Warm>;
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut volatility_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            inputs[0],
            self,
            &mut volatility_line,
        );

        Ok(vec![volatility_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub stddev_state: StddevState<S>,
    pub prev_real: f64,
}
impl State<Cold> {
    pub fn new(prev_real: f64, period: usize) -> Self {
        let stddev_state = StddevState::new(0.0, 0.0, period);
        let buffer = Buffer::new(period);
        State {
            prev_real,
            stddev_state,
            buffer,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut sum, mut sum_sq) = (0.0, 0.0);
        let mut buffer = Buffer::new(period);
        for i in 1..=period {
            let v = real[i] / real[i - 1] - 1.0;
            buffer.push(v);
            sum += v;
            sum_sq += v * v;
        }

        State {
            stddev_state: StddevState::new(sum, sum_sq, period).into_warm(),
            buffer: buffer.into_full(),
            prev_real: real[period],
        }
    }
    
    
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info(value);
        let (sd, _) = self.stddev_state.calc((value, prev_value));
        sd * ANNUAL
    }
}

/// Iterates over the real data slice and computes a Volatility value for each bar.
///
/// # Arguments
///
/// * `real` - Input data slice starting after the initialization window.
/// * `multiplier` - The stddev multiplier computed from the period.
/// * `state` - Mutable reference to the rolling calculation state.
/// * `vol_line` - Mutable output slice for volatility values.
fn cycle(real: &[f64], state: &mut State<Warm>, vol_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *vol_line.get_unchecked_mut(i) =
                state.calc(*real.get_unchecked(i));
        }
    }
}


pub struct Volatility;

impl Indicator<INPUTS, OPTIONS> for Volatility {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "volatility",
        full_name: "Volatility Indicator",
        indicator_type: IndicatorType::Volatility,
        inputs: &["real"],
        options: &["period"],
        outputs: &["volatility"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "volatility",
            label: "VOLATILITY",
            display_type: DisplayType::Indicator,
            outputs: &["volatility"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let mut vol_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::init_state(inputs[0], period);
    
        cycle(
            &inputs[0][period + 1..],
            &mut state,
            &mut vol_line,
        );
    
        Ok((vec![vol_line], state))
    }
}