use crate::common::{validate_inputs, validate_options};
use crate::indicators::ema::{calc as calc_ema, State as EmaState};

#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut mass_line = crate::uninit_vec!(f64, inputs[0].len());
        let [high, low] = inputs;
        cycle_mass(high, low, &mut mass_line, self);

        Ok(vec![mass_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub ema_state: EmaState<S>,
    pub ema_signal: f64,
    pub sum: f64,
}
impl<S> Deref for State<S> {
    type Target = EmaState<S>;
    fn deref(&self) -> &Self::Target {
        &self.ema_state
    }
}
impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ema_state
    }
}
impl State<Cold> {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        mass_line: &mut [f64],
    ) -> (usize, State<Warm>) {
        let (mut ema_state, mut ema_signal, mut buffer, mut sum) = (
            EmaState::new(high[0] - low[0], 9).into_warm(),
            0.0,
            Buffer::new(period),
            0.0,
        );
        let mut i = 1;
        while !buffer.is_full() {
            let hl_diff = high[i] - low[i];
            let ema = ema_state.calc(hl_diff);
            if i == 8 {
                ema_signal = ema;
            }
            if i >= 8 {
                ema_signal = calc_ema(
                    ema,
                    ema_signal,
                    ema_state.multiplier,
                    ema_state.inv_multiplier,
                );
                if i >= 16 {
                    let mass = (ema / ema_signal).max(0.0);
                    sum += mass;
                    buffer.push(mass);
                    if buffer.is_full() {
                        mass_line[0] = sum;
                    }
                }
            }
            i += 1;
        }
        (
            i,
            State {
                sum,
                ema_state,
                ema_signal,
                buffer: buffer.into_full(),
            },
        )
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> f64 {
        let hl_diff = (high - low).max(f64::EPSILON);

        let ema = self.ema_state.calc(hl_diff);
        self.ema_signal = calc_ema(ema, self.ema_signal, self.multiplier, self.inv_multiplier);
        let mass = (ema / self.ema_signal).max(0.0);
        self.sum += mass - self.buffer.push_with_info(mass);

        self.sum
    }
}
/// Performs the main calculation loop for the Mass indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `multipliers` - A tuple of EMA multipliers for the Mass calculation.
/// * `mass_line` - A mutable slice for storing the Mass output values.
/// * `state` - A mutable reference to the current `State`.
fn cycle_mass(high: &[f64], low: &[f64], mass_line: &mut [f64], state: &mut State<Warm>) {
    for i in 0..high.len() {
        unsafe {
            *mass_line.get_unchecked_mut(i) =
                state.calc((*high.get_unchecked(i), *low.get_unchecked(i)));
        }
    }
}

pub struct Mass;

impl Indicator<INPUTS, OPTIONS> for Mass {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "mass",
        indicator_type: IndicatorType::Trend,
        full_name: "Mass Index",
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["mass"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "mass",
            label: "MASS",
            display_type: DisplayType::Indicator,
            outputs: &["mass"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 16
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let mut mass_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let (high, low, mut state) = {
            let (start, state) =
                State::init_state(inputs[0], inputs[1], options[0] as usize, &mut mass_line);
            (&inputs[0][start..], &inputs[1][start..], state)
        };

        cycle_mass(high, low, &mut mass_line[1..], &mut state);

        Ok((vec![mass_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mass_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Mass {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mass_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
