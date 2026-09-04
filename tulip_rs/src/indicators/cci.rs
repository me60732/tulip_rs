use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::md::{Md, State as MdState};
use crate::indicators::typprice::calc as typprice_calc;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
//use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub md_state: MdState<S>,
}

impl State<Warm> {
    #[inline(always)]
    fn calc<const N: usize>(
        &mut self,
        (high, low, close): (f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let typprice = typprice_calc(high, low, close);
        //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);
        let old = self.buffer.push_with_info(typprice);

        let (md, sma) = self.md_state.calc((typprice, old, self.buffer.get_slice()));
        if md == 0.0 {
            return (0.0, sma, md, typprice);
        }
        let cci = (typprice - sma) / (0.015 * md);
        (cci, sma, md, typprice)
    }
}
impl State {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        (sma_line, md_line, typprice_line): (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let mut buffer = Buffer::new(period);
        let mut md_state = MdState::new(0.0, period);

        let mut i = 0;
        while !buffer.is_full() {
            let typprice = typprice_calc(high[i], low[i], close[i]);
            buffer.push(typprice);
            md_state.sum += typprice;
            crate::init_store_optional_outputs!(i, high.len(),
                typprice_line => typprice
            );
            i += 1;
        }

        let mut md_state = md_state.into_warm();
        let mut buffer = buffer.into_full();
        for i in period..period * 2 - 2 {
            let typprice = typprice_calc(high[i], low[i], close[i]);
            let old = buffer.push_with_info(typprice);
            let (md, sma) = md_state.calc((typprice, old, buffer.get_slice()));

            crate::init_store_optional_outputs!(i, high.len(),
                sma_line => sma,
                md_line => md,
                typprice_line => typprice
            );
        }
        State { buffer, md_state }
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, period: usize) -> Self {
        Self { state, period }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let capacity = inputs[0].len();
            (typprice_line, sma_line, md_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                typprice_line: capacity,
                sma_line: capacity,
                md_line: capacity
            );
            cci_line = crate::uninit_vec!(f64, capacity);
        };

        match self.period {
            1..=49 => cycle::<1>(
                (inputs[0], inputs[1], inputs[2]),
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            ),
            _ => cycle::<8>(
                (inputs[0], inputs[1], inputs[2]),
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            ),
        }

        Ok(vec![cci_line, sma_line, md_line, typprice_line])
    }
}

/// Performs the main calculation loop for the CCI indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `multiplier` - The CCI multiplier derived from the period (`1.0 / period`).
/// * `buffer` - Mutable reference to the indicator state (ring buffer and running sum).
/// * `cci_line` - Mutable slice to write the CCI output values into.
/// * `out_vecs` - A tuple of `(sma_line, md_line, typprice_line)` for optional outputs.
fn cycle<const N: usize>(
    (high, low, close): (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    cci_line: &mut [f64],
    (sma_line, md_line, typprice_line): (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (has_optional, want_typ, want_sma, want_md) =
        crate::calc_want_flags!(typprice_line, sma_line, md_line);

    //high.iter().zip(low.iter()).zip(close.iter()).skip(start).enumerate().for_each(|(i, ((h, l), c))| {
    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (cci, sma, md, typprice) = state.calc::<N>(inputs);

        unsafe { *cci_line.get_unchecked_mut(i) = cci };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_sma, sma_line => sma,
                want_md, md_line => md,
                want_typ, typprice_line => typprice
            );
        }
    }
}

pub struct Cci;

impl Indicator<INPUTS, OPTIONS> for Cci {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "cci",
        indicator_type: IndicatorType::Momentum,
        full_name: "Commodity Channel Index",
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["cci"],
        optional_outputs: &["sma", "md", "typprice"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "cci",
                label: "CCI",
                display_type: DisplayType::Indicator,
                outputs: &["cci"],
            },
            DisplayGroup {
                offset: None,
                id: "sma_typprice",
                label: "Typical Price",
                display_type: DisplayType::Overlay,
                outputs: &["sma", "typprice"],
            },
            DisplayGroup {
                offset: None,
                id: "md",
                label: "Mean Deviation",
                display_type: DisplayType::Indicator,
                outputs: &["md"],
            },
        ],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize * 2 - 1
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let capacity = Self::output_length(high.len(), options);
            let md_capacity = Md::output_length(high.len(), options);
            cci_line = crate::uninit_vec!(f64, capacity);
            (sma_line, md_line, typprice_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                sma_line: md_capacity,
                md_line: md_capacity,
                typprice_line: high.len()
            );
        };

        let mut state = State::init_state(
            high,
            low,
            close,
            period,
            (&mut sma_line, &mut md_line, &mut typprice_line),
        );
        let optional_outputs = {
            let offset =
                crate::slice_outputs_start!(cci_line.len(), sma_line, md_line, typprice_line);
            (
                &mut sma_line[offset.0..],
                &mut md_line[offset.1..],
                &mut typprice_line[offset.2..],
            )
        };
        let inputs = {
            let from = period * 2 - 2;
            (&high[from..], &low[from..], &close[from..])
        };
        match period {
            1..=49 => cycle::<1>(inputs, &mut state, &mut cci_line, optional_outputs),
            _ => cycle::<8>(inputs, &mut state, &mut cci_line, optional_outputs),
        }

        Ok((
            vec![cci_line, sma_line, md_line, typprice_line],
            IndicatorState::new(state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cci_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Cci {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cci_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
