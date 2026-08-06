use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, TState, TSimdState, Indicator, IndicatorResult};
pub(crate) use crate::indicators::cmo::up_down;
use crate::indicators::simd_indicators::wilders_simd::{
    multiplier_simd, SimdState as WildersSimdState
};
pub use crate::indicators::wilders::multiplier;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
use std::simd::Simd;
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::rsi_simd::indicator_by_options as indicator;
}

pub type IndicatorState = State<Warm>;

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut rsi_line = crate::uninit_vec!(f64, inputs[0].len());
        cycle_rsi(inputs[0], &mut rsi_line, self);

        Ok(vec![rsi_line])
    }
}

#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub wilders_state: WildersSimdState<2>,
    pub prev_real: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(prev_real: f64, up_sum: f64, down_sum: f64, period: usize) -> Self {
        let multipliers = multiplier_simd([period, period]);
        Self {
            prev_real,
            wilders_state: WildersSimdState::new(Simd::from_array([up_sum, down_sum]), multipliers),
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let [up, down] = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        up_sum /= period as f64;
        down_sum /= period as f64;

        State {
            prev_real: real[period],
            wilders_state: WildersSimdState::new(
                Simd::from_array([up_sum, down_sum]),
                multiplier_simd([period, period]),
            ),
            state: std::marker::PhantomData,
        }
    }
    
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, cur_real: Self::Inputs<'a>) -> Self::Outputs {
        let up_down = up_down(cur_real, self.prev_real);
        let [up_sum, down_sum] = self
            .wilders_state
            .calc(Simd::from_array(up_down))
            .to_array();

        self.prev_real = cur_real;

        100.0 * (up_sum / (up_sum + down_sum))
    }
}



/// Performs the main calculation loop for the RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `multiplier` - The smoothing multiplier for the RSI calculation.
/// * `rsi_line` - A mutable slice for storing the RSI output values.
/// * `state` - A mutable reference to the current RSI `State`.
fn cycle_rsi(real: &[f64], rsi_line: &mut [f64], state: &mut State<Warm>) {
    for i in 0..real.len() {
        unsafe { *rsi_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i)) };
    }
}

pub struct Rsi;
impl Indicator<INPUTS, OPTIONS> for Rsi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "rsi",
        indicator_type: IndicatorType::Momentum,
        full_name: "Relative Strength Index",
        inputs: &["real"],
        options: &["period"],
        outputs: &["rsi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "rsi",
            label: "RSI",
            display_type: DisplayType::Indicator,
            outputs: &["rsi"],
        }],
    };

    fn output_length(data_len: usize, options: &[f64; OPTIONS]) -> usize {
        data_len - Self::min_data(options)
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let mut rsi_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
    
        let mut state = State::init_state(inputs[0], period);
    
        cycle_rsi(&inputs[0][period + 1..], &mut rsi_line, &mut state);
    
        Ok((vec![rsi_line], state))
    }
}