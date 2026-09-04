use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::ema::{multiplier as ema_multiplier, Ema, State as EmaState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_ema: EmaState<S>,
    pub long_ema: EmaState<S>,
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let capacity = inputs[0].len();
        let mut apo_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: capacity,
            long_ema_line: capacity
        );

        cycle_apo(
            inputs[0],
            self,
            &mut apo_line,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![apo_line, short_ema_line, long_ema_line])
    }
}

impl State<Cold> {
    pub fn new(ema: f64, short_period: usize, long_period: usize) -> Self {
        Self {
            short_ema: EmaState::new(ema, short_period),
            long_ema: EmaState::new(ema, long_period),
        }
    }
    /*pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            short_ema: self.short_ema.into_warm(),
            long_ema: self.long_ema.into_warm(),
        }
    }*/
    pub fn init_state(
        real: &[f64],
        short_period: usize,
        long_period: usize,
        short_ema_line: &mut [f64],
    ) -> State<Warm> {
        let mut short_ema = EmaState::new(real[0], short_period).into_warm();
        let mut long_ema = EmaState::new(real[0], long_period).into_warm();

        for (i, &value) in real.iter().enumerate().take(long_period - 1).skip(1) {
            let s_ema = short_ema.calc(value);
            long_ema.calc(value);
            crate::init_store_optional_outputs!(i, real.len(),
                short_ema_line => s_ema
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
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let short_ema = self.short_ema.calc(real);
        let long_ema = self.long_ema.calc(real);
        short_ema - long_ema
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Absolute Price Oscillator (APO) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close prices
///
/// # Options
///
/// * `options[0]` — short period (must be >= 1)
/// * `options[1]` — long period (must be > short period)
///
/// # Arguments
///
/// * `inputs` - Array of 1 input price slice (see Inputs above).
/// * `options` - Array of 2 indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable individual
///   optional outputs (`short_ema`, `long_ema`); `None` disables all.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `apo` line,
/// `outputs[1]` is the optional `short_ema` line, and `outputs[2]` is the optional `long_ema` line
/// (each empty if not requested).
/// `state` can be passed to `IndicatorState::batch_indicator` to continue streaming.
///
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.

/// Performs the main calculation loop for the APO indicator.
///
/// # Arguments
///
/// * `real` - A slice of close prices to process.
/// * `state` - A mutable reference to the current `State` (short EMA, long EMA).
/// * `multipliers` - The precomputed EMA multipliers for the short and long periods.
/// * `apo_line` - A mutable slice for storing the resulting APO line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: short EMA and long EMA lines.
fn cycle_apo(
    real: &[f64],
    state: &mut State<Warm>,
    apo_line: &mut [f64],
    (short_ema_line, long_ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        unsafe { *apo_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i)) };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => state.short_ema.ema,
                want_long, long_ema_line => state.long_ema.ema
            );
        }
    }
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}

pub struct Apo;

impl Indicator<INPUTS, OPTIONS> for Apo {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "apo",
        full_name: "Absolute Price Oscillator",
        indicator_type: IndicatorType::Momentum,
        inputs: &["close"],
        options: &["short_period", "long_period"],
        outputs: &["apo"],
        optional_outputs: &["short_ema", "long_ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "apo",
                label: "APO",
                display_type: DisplayType::Indicator,
                outputs: &["apo"],
            },
            DisplayGroup {
                offset: None,
                id: "short_ema_long_ema",
                label: "EMA",
                display_type: DisplayType::Overlay,
                outputs: &["short_ema", "long_ema"],
            },
        ],
    };
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[1] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        let short_period = options[0] as usize;
        let long_period = options[1] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        let real = inputs[0];

        let capacity = Self::output_length(real.len(), options);
        let short_ema_capacity = Ema::output_length(real.len(), &[short_period as f64]);

        let mut apo_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: capacity
        );

        let mut state = State::init_state(real, short_period, long_period, &mut short_ema_line);

        let optional_outputs = {
            let short_start = crate::slice_outputs_start!(capacity, short_ema_line);
            (
                &mut short_ema_line[short_start..],
                long_ema_line.as_mut_slice(),
            )
        };

        cycle_apo(
            &real[real.len() - apo_line.len()..],
            &mut state,
            &mut apo_line,
            optional_outputs,
        );

        Ok((vec![apo_line, short_ema_line, long_ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::apo_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Apo {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::apo_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
