use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
//use crate::indicators::atr::calc as calc_atr;
pub use crate::indicators::atr::{multiplier, State as AtrState};
use crate::indicators::tr::Tr;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub AtrState<S>);
impl<S> Deref for State<S> {
    type Target = AtrState<S>;
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
pub type IndicatorState = State<Warm>;

impl State<Cold> {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        State(AtrState::init_state(
            high, low, close, period, tr_line, false,
        ))
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let (atr, tr) = self.0.calc((high, low, close));
        ((atr / close) * 100.0, atr, tr)
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut natr_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            natr_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        cycle_natr(
            (inputs[0], inputs[1], inputs[2]),
            &mut natr_line,
            (&mut atr_line, &mut tr_line),
            self,
        );

        Ok(vec![natr_line, atr_line, tr_line])
    }
}

/// Iterates over the input data and applies the calc function.
//#[inline(always)]
fn cycle_natr(
    (high, low, close): (&[f64], &[f64], &[f64]),
    natr_line: &mut [f64],
    (atr_line, tr_line): (&mut [f64], &mut [f64]),
    state: &mut State<Warm>,
) {
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (natr, atr, tr) = state.calc(inputs);
        unsafe { *natr_line.get_unchecked_mut(i) = natr };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr
            );
        }
    }
}

pub struct Natr;

impl Indicator<INPUTS, OPTIONS> for Natr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "natr",
        full_name: "Normalized Average True Range",
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["natr"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "natr",
                label: "NATR",
                display_type: DisplayType::Indicator,
                outputs: &["natr"],
            },
            DisplayGroup {
                offset: None,
                id: "atr_tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["atr", "tr"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let (mut natr_line, mut atr_line, mut tr_line);
        {
            let capacity = Self::output_length(inputs[0].len(), options);
            natr_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: Tr::output_length(inputs[0].len(), &[])
            );
        }
        let mut state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line);
        let offset = crate::slice_outputs_start!(natr_line.len(), tr_line);

        cycle_natr(
            (
                &inputs[0][period..],
                &inputs[1][period..],
                &inputs[2][period..],
            ),
            &mut natr_line,
            (&mut atr_line, &mut tr_line[offset..]),
            &mut state,
        );

        Ok((vec![natr_line, atr_line, tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::natr_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Natr {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::natr_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
