use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::dx::{Dx, State as DxState};
use crate::indicators::tr::Tr;
pub use crate::indicators::wilders::multiplier;
use crate::indicators::wilders::State as WildersState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub dx_state: DxState<S>,
    pub wilders_state: WildersState<S>,
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> (f64, f64, f64, f64) {
        let (dx, atr, tr) = self.dx_state.calc(inputs);
        let adx = self.wilders_state.calc(dx);
        (adx, dx, atr, tr)
    }
}
impl State {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (dx_line, atr_line, tr_line) = out_vecs;
        let (_, inv_multiplier) = multiplier(period);
        let mut dx_state = DxState::init_state(high, low, close, period, tr_line);
        let mut adx = dx_state.calc_dx();
        for (i, ((&h, &l), &c)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .take(period * 2 - 1)
            .skip(period)
        {
            let (dx, atr, tr) = dx_state.calc((h, l, c));
            adx += dx;
            crate::init_store_optional_outputs!(i, high.len(),
                dx_line => dx,
                atr_line => atr * inv_multiplier,
                tr_line => tr
            );
        }
        adx /= period as f64;
        State {
            dx_state,
            wilders_state: WildersState::new(adx, period).into_warm(),
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut adx_line, mut dx_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();

            adx_line = crate::uninit_vec!(f64, capacity);
            (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                dx_line: capacity,
                atr_line: capacity,
                tr_line: capacity
            );
        }

        cycle_adx(
            (inputs[0], inputs[1], inputs[2]),
            self,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );

        Ok(vec![adx_line, dx_line, atr_line, tr_line])
    }
}

/// Performs the main calculation loop for the ADX indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `period` - The period for the ADX calculation.
/// * `indicator_state` - A slice containing necessary input values.
/// * `start` - The starting index for the calculation.
/// * `adx_line` - A mutable reference to a vector for storing the ADX line.
/// * `output_vectors` - A mutable reference to an array of optional vectors for storing additional outputs.
//#[inline(always)]
fn cycle_adx(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (adx_line, dx_line, atr_line, tr_line) = out_vecs;

    let (has_optional, want_dx, want_atr, want_tr) =
        crate::calc_want_flags!(dx_line, atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (adx, dx, atr, tr) = state.calc(inputs);
        unsafe {
            *adx_line.get_unchecked_mut(i) = adx;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_dx, dx_line => dx,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, state.dx_state.atr_state.wilders_state.inv_multiplier)
            );
        }
    }
}

pub struct Adx;

impl Indicator<INPUTS, OPTIONS> for Adx {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "adx",
        full_name: "Average Directional Index",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["adx"],
        optional_outputs: &["dx", "atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "adx_dx",
                label: "Directional Index",
                display_type: DisplayType::Indicator,
                outputs: &["adx", "dx"],
            },
            DisplayGroup {
                offset: None,
                id: "true_range",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["atr", "tr"],
            },
        ],
    };
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize * 2 // period
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        let period = options[0] as usize;
        validate_inputs(inputs, Self::min_data(options))?;

        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let (mut adx_line, mut dx_line, mut atr_line, mut tr_line);
        {
            let dx_capacity = Dx::output_length(inputs[0].len(), options);
            let adx_capacity = Self::output_length(inputs[0].len(), options);
            let tr_capacity = Tr::output_length(inputs[0].len(), &[]);
            adx_line = crate::uninit_vec!(f64, adx_capacity);

            (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                dx_line: dx_capacity,
                atr_line: dx_capacity,
                tr_line: tr_capacity
            );
        }

        let mut state = State::init_state(
            high,
            low,
            close,
            period,
            (&mut dx_line, &mut atr_line, &mut tr_line),
        );
        let outputs = {
            let offsets = crate::slice_outputs_start!(adx_line.len(), dx_line, atr_line, tr_line);
            (
                adx_line.as_mut_slice(),
                &mut dx_line[offsets.0..],
                &mut atr_line[offsets.1..],
                &mut tr_line[offsets.2..],
            )
        };
        let inputs = {
            let from = period * 2 - 1;
            (&high[from..], &low[from..], &close[from..])
        };
        cycle_adx(inputs, &mut state, outputs);

        Ok((vec![adx_line, dx_line, atr_line, tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adx_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Adx {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adx_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
