use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::dema::Dema;
use crate::indicators::ema::Ema;
use crate::indicators::tema::{State as TemaState, Tema};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub TemaState<S>);
impl<S> Deref for State<S> {
    type Target = TemaState<S>;
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
impl State<Cold> {
    pub fn init_state(
        real: &[f64],
        period: usize,
        trix_capacity: usize,
        (tema_line, dema_line, ema_line): (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let remaining = real.len() - trix_capacity;
        let tema_capacity = Tema::output_length(real.len(), &[period as f64]);
        let mut state = State(TemaState::init_state(real, period, (dema_line, ema_line)));
        let mut i = real.len() - tema_capacity;

        while i < remaining {
            let (tema, dema, ema) = state.0.calc(real[i]);

            crate::init_store_optional_outputs!(i, real.len(),
                tema_line => tema,
                dema_line => dema,
                ema_line => ema
            );
            i += 1;
        }
        state
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        let prev_ema3 = self.ema3;
        let (tema, dema, ema) = self.0.calc(value);
        // Compute TRIX as percentage change if previous TEMA is non-zero.
        let trix = 100.0 * (self.ema3 - prev_ema3) / self.ema3;

        (trix, tema, dema, ema)
    }
}

pub type IndicatorState = State<Warm>;

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut trix_line, mut tema_line, mut dema_line, mut ema_line);
        {
            let capacity = inputs[0].len();
            trix_line = crate::uninit_vec!(f64, capacity);
            (tema_line, dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                tema_line: capacity,
                dema_line: capacity,
                ema_line: capacity
            );
        }
        cycle_trix(
            inputs[0],
            self,
            &mut trix_line,
            (&mut tema_line, &mut dema_line, &mut ema_line),
        );

        Ok(vec![trix_line, tema_line, dema_line, ema_line])
    }
}

/// Performs the main calculation loop for the TRIX indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `multipliers` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
/// * `state` - A mutable reference to the current TEMA indicator state.
/// * `trix_line` - A mutable slice for storing the TRIX output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(tema_line, dema_line, ema_line)`.
fn cycle_trix(
    real: &[f64],
    state: &mut State<Warm>,
    trix_line: &mut [f64],
    (tema_line, dema_line, ema_line): (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (has_optional, want_tema, want_dema, want_ema) =
        crate::calc_want_flags!(tema_line, dema_line, ema_line);

    for i in 0..real.len() {
        let (tema, dema, ema);
        unsafe {
            (*trix_line.get_unchecked_mut(i), tema, dema, ema) = state.calc(*real.get_unchecked(i))
        };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_tema, tema_line => tema,
                want_dema, dema_line => dema,
                want_ema, ema_line => ema
            );
        }
    }
}

pub struct Trix;

impl Indicator<INPUTS, OPTIONS> for Trix {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "trix",
        full_name: "Triple Exponential Oscillator (TRIX)",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trix"],
        optional_outputs: &["tema", "dema", "ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "trix",
                label: "TRIX",
                display_type: DisplayType::Indicator,
                outputs: &["trix"],
            },
            DisplayGroup {
                offset: None,
                id: "tema_dema_ema",
                label: "EMAs",
                display_type: DisplayType::Overlay,
                outputs: &["tema", "dema", "ema"],
            },
        ],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        let period = options[0] as usize;
        (period - 1) * 3 + 2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let (mut trix_line, mut tema_line, mut dema_line, mut ema_line, mut state, real);
        {
            let len = inputs[0].len();
            let capacity = Self::output_length(len, options);
            let tema_cap = Tema::output_length(len, options);
            let dema_cap = Dema::output_length(len, options);
            let ema_cap = Ema::output_length(len, options);

            // Initialize output storage: main TRIX line plus optional outputs (TEMA, DEMA, EMA)
            trix_line = crate::uninit_vec!(f64, capacity);
            (tema_line, dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                tema_line: tema_cap,
                dema_line: dema_cap,
                ema_line: ema_cap
            );
            let period = options[0] as usize;
            state = State::init_state(
                inputs[0],
                period,
                capacity,
                (&mut tema_line, &mut dema_line, &mut ema_line),
            );
            let start = len - capacity;
            real = &inputs[0][start..]
        }
        let optional_outputs = {
            let offsets =
                crate::slice_outputs_start!(trix_line.len(), tema_line, dema_line, ema_line);
            (
                &mut tema_line[offsets.0..],
                &mut dema_line[offsets.1..],
                &mut ema_line[offsets.2..],
            )
        };

        cycle_trix(real, &mut state, &mut trix_line, optional_outputs);

        Ok((vec![trix_line, tema_line, dema_line, ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trix_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Trix {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trix_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
