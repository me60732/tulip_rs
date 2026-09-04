use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
pub use crate::indicators::adx::multiplier;
use crate::indicators::adx::{Adx, State as AdxState};

use crate::indicators::dx::Dx;
use crate::indicators::tr::Tr;
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, Cold, Warm};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub adx_state: AdxState<S>,
    pub buffer: Buffer<S, f64>,
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> (f64, f64, f64, f64, f64) {
        let (adx, dx, atr, tr) = self.adx_state.calc(inputs);
        let adxr = 0.5 * (adx + self.buffer.push_with_info(adx));

        (adxr, adx, dx, atr, tr)
    }
}
impl State<Cold> {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (adx_line, dx_line, atr_line, tr_line) = out_vecs;
        let mut adx_state =
            AdxState::init_state(high, low, close, period, (dx_line, atr_line, tr_line));
        let mut prev_adx = Buffer::new(period - 1);
        prev_adx.push(adx_state.wilders_state.wilders);

        let mut i = period * 2 - 1;
        let multipliers = multiplier(period);
        while !prev_adx.is_full() {
            let (adx, dx, atr, tr) = adx_state.calc((high[i], low[i], close[i]));
            prev_adx.push(adx);
            crate::init_store_optional_outputs!(i, high.len(),
                adx_line => adx,
                dx_line => dx,
                atr_line => atr * multipliers.1,
                tr_line => tr
            );
            i += 1;
        }
        State {
            adx_state,
            buffer: prev_adx.into_full(),
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

        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let capacity = inputs[0].len();
        //let mut adxr_line = vec![0.0; capacity]; //Vec::with_capacity(capacity);
        let mut adxr_line: Vec<f64> = Vec::with_capacity(capacity);
        unsafe {
            adxr_line.set_len(capacity);
        }
        let (mut adx_line, mut dx_line, mut atr_line, mut tr_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false, false],
            adx_line: capacity,
            dx_line: capacity,
            atr_line: capacity,
            tr_line: capacity
        );

        cycle_adxr(
            &high,
            &low,
            &close,
            self,
            &mut adxr_line,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );
        Ok(vec![adxr_line, adx_line, dx_line, atr_line, tr_line])
    }
}

/// Performs the main calculation loop for the ADXR indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - A mutable reference to the current `State` (ADX state and the rolling ADX buffer).
/// * `inv_multiplier` - The inverse ATR multiplier used to scale ATR values.
/// * `adxr_line` - A mutable slice for storing the resulting ADXR line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: ADX, DX, ATR, and TR lines.
//#[inline(always)]
fn cycle_adxr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State<Warm>,
    adxr_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (adx_line, dx_line, atr_line, tr_line) = out_vecs;
    let (has_optional, want_adx, want_dx, want_atr, want_tr) =
        crate::calc_want_flags!(adx_line, dx_line, atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (adxr, adx, dx, atr, tr) = state.calc(inputs);

        unsafe {
            *adxr_line.get_unchecked_mut(i) = adxr;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_adx, adx_line => adx,
                want_dx, dx_line => dx,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, state.adx_state.dx_state.atr_state.wilders_state.inv_multiplier)
            );
        }
    }
}

pub struct Adxr;

impl Indicator<INPUTS, OPTIONS> for Adxr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "adxr",
        full_name: "Average Directional Movement Rating",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["adxr"],
        optional_outputs: &["adx", "dx", "atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "adxr_adx_dx",
                label: "Directional Index",
                display_type: DisplayType::Indicator,
                outputs: &["adxr", "adx", "dx"],
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
        (options[0] as usize - 1) * 3 + 1 // period
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        validate_inputs(inputs, Self::min_data(options))?;

        /*let mut adxr_line: Vec<f64> = Vec::with_capacity(adxr_capacity);
        unsafe { adxr_line.set_len(adxr_capacity); }*/
        //let mut adxr_line = vec![0.0; adxr_capacity]; // Vec::with_capacity(adxr_capacity);
        let (mut adxr_line, (mut adx_line, mut dx_line, mut atr_line, mut tr_line)) = {
            let len = inputs[0].len();
            let adxr_capacity = Self::output_length(len, options);
            let adx_capacity = Adx::output_length(len, options);
            let dx_capacity = Dx::output_length(len, options);
            let tr_capacity = Tr::output_length(len, &[]);

            (
                crate::uninit_vec!(f64, adxr_capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false, false],
                    adx_line: adx_capacity,
                    dx_line: dx_capacity,
                    atr_line: dx_capacity,
                    tr_line: tr_capacity
                ),
            )
        };

        let mut state = State::init_state(
            inputs[0], // high
            inputs[1], //low
            inputs[2], //close
            period,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );
        let (high, low, close) = {
            let from = inputs[0].len() - adxr_line.len();
            (&inputs[0][from..], &inputs[1][from..], &inputs[2][from..])
        };
        let outputs = {
            let offsets =
                crate::slice_outputs_start!(adxr_line.len(), adx_line, dx_line, atr_line, tr_line);
            (
                &mut adx_line[offsets.0..],
                &mut dx_line[offsets.1..],
                &mut atr_line[offsets.2..],
                &mut tr_line[offsets.3..],
            )
        };

        cycle_adxr(high, low, close, &mut state, &mut adxr_line, outputs);

        Ok((vec![adxr_line, adx_line, dx_line, atr_line, tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adxr_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Adxr {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adxr_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
