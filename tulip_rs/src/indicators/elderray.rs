use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::ema::State as EmaState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
//use wide::*;

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub EmaState<S>);
impl<S> Deref for State<S> {
    type Target = EmaState<S>;
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
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let ema = self.0.calc(close);

        (high - ema, low - ema, ema)
    }
}
impl State {
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        State(EmaState::init_state(real, period))
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let inputs = (inputs[0], inputs[1], inputs[2]);

        let (mut bull_line, mut bear_line, mut ema_line) = {
            let len = inputs.0.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: len
                ),
            )
        };
        cycle(
            inputs,
            self,
            (&mut bull_line, &mut bear_line, &mut ema_line),
        );
        Ok(vec![bull_line, bear_line, ema_line])
    }
}

fn cycle(
    (high, low, close): (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    (bull_line, bear_line, ema_line): (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (_, want_ema) = crate::calc_want_flags!(ema_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (bull, bear, ema) = state.calc(inputs);
        unsafe {
            *bull_line.get_unchecked_mut(i) = bull;
            *bear_line.get_unchecked_mut(i) = bear;
        };

        crate::store_optional_outputs!(i,
            want_ema, ema_line => ema
        );
    }
}

pub struct Elderray;

impl Indicator<INPUTS, OPTIONS> for Elderray {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "elderray",
        full_name: "Elder-ray",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["bull", "bear"],
        optional_outputs: &["ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "elderray",
                label: "Elder-ray",
                display_type: DisplayType::Indicator,
                outputs: &["bull", "bear"],
            },
            DisplayGroup {
                offset: None,
                id: "ema",
                label: "EMA",
                display_type: DisplayType::Overlay,
                outputs: &["ema"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;

        let (mut bull_line, mut bear_line, mut ema_line, inputs, mut state) = {
            let capacity = Self::output_length(inputs[0].len(), options);
            let period = options[0] as usize;
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                ),
                (
                    &inputs[0][period..],
                    &inputs[1][period..],
                    &inputs[2][period..],
                ),
                State::init_state(inputs[2], period),
            )
        };

        cycle(
            inputs,
            &mut state,
            (&mut bull_line, &mut bear_line, &mut ema_line),
        );
        Ok((vec![bull_line, bear_line, ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::elderray_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Elderray {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::elderray_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
