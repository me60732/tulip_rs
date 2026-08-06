use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;
pub const OUTPUTS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ad_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ad_simd::indicator_by_assets as indicator;
}

pub type State = IndicatorState;
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub ad: f64,
}
impl IndicatorState {
    pub fn new(ad: f64) -> Self {
        Self { ad }
    }
}
impl TState for State {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
        let range = high - low;
        if range <= f64::EPSILON {
            return self.ad;
        }

        //ad + (close - low - high + close) / range * volume
        self.ad = ((close - low - high + close) / range).mul_add(volume, self.ad);
        self.ad
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    //#[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut ad_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs, &mut ad_line, self);

        Ok(vec![ad_line])
    }
}

/// Performs the main calculation loop for the AD indicator.
///
/// # Arguments
///
/// * `inputs` - A reference to an array of 4 input slices: high, low, close, and volume.
/// * `ad_line` - A mutable slice for storing the resulting AD line values.
/// * `ad` - The running AD accumulator value to continue from.
///
/// # Returns
///
/// The final AD accumulator value after processing all inputs.
fn cycle([high, low, close, volume]: &[&[f64]; INPUTS], ad_line: &mut [f64], state: &mut State) {
    for i in 0..high.len() {
        unsafe {
            *ad_line.get_unchecked_mut(i) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            ));
        };
    }
}

pub struct Ad;
impl Indicator<INPUTS, OPTIONS> for Ad {
    const INFO: Info = Info {
        name: "ad",
        full_name: "Accumulation/Distribution Line",
        indicator_type: IndicatorType::Volume,
        inputs: &["high", "low", "close", "volume"],
        options: &[],
        outputs: &["ad"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "ad",
            label: "AD",
            display_type: DisplayType::Indicator,
            outputs: &["ad"],
        }],
    };

    type IndicatorState = IndicatorState;

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        1
    }
    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_inputs(inputs, Self::min_data(_options))?;

        let mut ad_line = crate::uninit_vec!(f64, inputs[0].len());

        let mut state = State::new(0.0);
        cycle(inputs, &mut ad_line, &mut state);

        Ok((vec![ad_line], state))
    }
}
