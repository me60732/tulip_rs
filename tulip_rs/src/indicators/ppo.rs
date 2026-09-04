use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::ema::{Ema, State as EmaState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real = inputs[0];

        let (mut ppo_line, mut short_ema_line, mut long_ema_line);
        {
            let capacity = real.len();
            ppo_line = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: capacity,
                long_ema_line: capacity
            );
        }
        cycle_ppo(
            real,
            &mut ppo_line,
            self,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![ppo_line, short_ema_line, long_ema_line])
    }
}
pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_ema: EmaState<S>,
    pub long_ema: EmaState<S>,
}
impl State {
    pub fn new(short_ema: f64, long_ema: f64, (short_period, long_period): (usize, usize)) -> Self {
        State {
            short_ema: EmaState::new(short_ema, short_period),
            long_ema: EmaState::new(long_ema, long_period),
        }
    }
    pub fn init_state(
        real: &[f64],
        (short_period, long_period): (usize, usize),
        short_ema_line: &mut [f64],
    ) -> State<Warm> {
        let mut short_ema = EmaState::new(real[0], short_period).into_warm();
        let mut long_ema = EmaState::new(real[0], long_period).into_warm();
        for i in 1..long_period {
            let short_ema = short_ema.calc(real[i]);
            long_ema.calc(real[i]);
            crate::init_store_optional_outputs!(i, real.len(),
                short_ema_line => short_ema
            );
        }

        State {
            short_ema,
            long_ema,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64, f64);

    /// Performs the core calculation for the Percentage Price Oscillator (PPO) indicator.
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let short_ema = self.short_ema.calc(real);
        let long_ema = self.long_ema.calc(real).max(f64::EPSILON);

        (
            (short_ema - long_ema) * 100.0 / long_ema,
            short_ema,
            long_ema,
        )
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Iterates over the input data and applies the calc function.
fn cycle_ppo(
    real: &[f64],
    ppo_line: &mut [f64],
    state: &mut State<Warm>,
    (short_ema_line, long_ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        let value = unsafe { *real.get_unchecked(i) };

        let (ppo, short_ema, long_ema) = state.calc(value);

        unsafe { *ppo_line.get_unchecked_mut(i) = ppo };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => short_ema,
                want_long, long_ema_line => long_ema
            );
        }
    }
}

pub struct Ppo;

impl Indicator<INPUTS, OPTIONS> for Ppo {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ppo",
        full_name: "Percentage Price Oscillator",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["short_period", "long_period"],
        outputs: &["ppo"],
        optional_outputs: &["short_ema", "long_ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "ppo",
                label: "PPO",
                display_type: DisplayType::Indicator,
                outputs: &["ppo"],
            },
            DisplayGroup {
                offset: None,
                id: "short_ema_long_ema",
                label: "EMAs",
                display_type: DisplayType::Overlay,
                outputs: &["short_ema", "long_ema"],
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

        let (mut ppo_line, mut short_ema_line, mut long_ema_line, mut state, real);
        {
            let short_period = options[0] as usize;
            let long_period = options[1] as usize;
            let capacity = Self::output_length(inputs[0].len(), options);
            let short_ema_capacity = Ema::output_length(inputs[0].len(), &[short_period as f64]);

            ppo_line = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: short_ema_capacity,
                long_ema_line: capacity
            );

            state = State::init_state(inputs[0], (short_period, long_period), &mut short_ema_line);
            real = &inputs[0][long_period..];
        }
        let optional_outputs = {
            let offset = crate::slice_outputs_start!(ppo_line.len(), short_ema_line);
            (&mut short_ema_line[offset..], long_ema_line.as_mut_slice())
        };

        cycle_ppo(real, &mut ppo_line, &mut state, optional_outputs);

        Ok((vec![ppo_line, short_ema_line, long_ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ppo_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Ppo {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ppo_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
