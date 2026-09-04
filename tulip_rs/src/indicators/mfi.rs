use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::typprice::calc as calc_typprice;
use crate::ring_buffer::multi_buffer::multi_buffer::MultiBuffer as Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut mfi_line, mut typprice_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    typprice_line: capacity
                ),
            )
        };

        cycle_mfi(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            self,
            &mut mfi_line,
            &mut typprice_line,
        );

        Ok(vec![mfi_line, typprice_line])
    }
}
pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<2, f64, S>,
    pub typprice: f64,
    pub pos_sum: f64,
    pub neg_sum: f64,
}
impl State<Cold> {
    pub fn init_state(
        inputs: (&[f64], &[f64], &[f64], &[f64]),
        period: usize,
        typprice_line: &mut [f64],
    ) -> State<Warm> {
        let (high, low, close, volume) = inputs;
        let mut state = Self {
            typprice: calc_typprice(high[0], low[0], close[0]),
            pos_sum: 0.0,
            neg_sum: 0.0,
            buffer: Buffer::new(period),
        };

        for i in 0..period {
            state.init_calc((high[i], low[i], close[i], volume[i]));
            crate::init_store_optional_outputs!(i, high.len(),
                typprice_line => state.typprice
            );
        }
        State {
            buffer: state.buffer.into_full(),
            typprice: state.typprice,
            pos_sum: state.pos_sum,
            neg_sum: state.neg_sum,
        }
    }
    #[inline(always)]
    fn init_calc(&mut self, (high, low, close, volume): (f64, f64, f64, f64)) {
        let prev_typprice = self.typprice;
        self.typprice = calc_typprice(high, low, close);

        let price_change = self.typprice - prev_typprice;

        let (pos_flow, neg_flow) = if price_change > 0.0 {
            (self.typprice * volume, 0.0)
        } else if price_change < 0.0 {
            (0.0, self.typprice * volume)
        } else {
            (0.0, 0.0)
        };

        if let Some([pos_flow_old, neg_flow_old]) = self.buffer.push_with_info([pos_flow, neg_flow])
        {
            self.pos_sum += pos_flow - pos_flow_old;
            self.neg_sum += neg_flow - neg_flow_old;
        } else {
            self.pos_sum += pos_flow;
            self.neg_sum += neg_flow
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
        let prev_typprice = self.typprice;
        self.typprice = calc_typprice(high, low, close);

        let price_change = self.typprice - prev_typprice;
        let money_flow = self.typprice * volume;

        let (pos_flow, neg_flow) = if price_change > 0.0 {
            (money_flow, 0.0)
        } else if price_change < 0.0 {
            (0.0, money_flow)
        } else {
            (0.0, 0.0)
        };

        let [pos_flow_old, neg_flow_old] = self.buffer.push_with_info([pos_flow, neg_flow]);
        self.pos_sum += pos_flow - pos_flow_old;
        self.neg_sum += neg_flow - neg_flow_old;

        self.pos_sum / (self.pos_sum + self.neg_sum).max(f64::EPSILON) * 100.0
    }
}

/// Performs the main calculation loop for the MFI indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of slices for high, low, close, and volume data.
/// * `state` - A mutable reference to the current `IndicatorState`.
/// * `mfi_line` - A mutable slice for storing the MFI output values.
/// * `typprice_line` - A mutable slice for storing optional typical price output values.
fn cycle_mfi(
    (high, low, close, volume): (&[f64], &[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    mfi_line: &mut [f64],
    typprice_line: &mut [f64],
) {
    let (_, want_typprice) = crate::calc_want_flags!(typprice_line);

    for i in 0..high.len() {
        unsafe {
            *mfi_line.get_unchecked_mut(i) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            ));
        }
        crate::store_optional_outputs!(i,
            want_typprice, typprice_line => state.typprice
        );
    }
}

pub struct Mfi;

impl Indicator<INPUTS, OPTIONS> for Mfi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "mfi",
        indicator_type: IndicatorType::Volume,
        full_name: "Money Flow Index",
        inputs: &["high", "low", "close", "volume"],
        options: &["period"],
        outputs: &["mfi"],
        optional_outputs: &["typprice"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "mfi",
                label: "MFI",
                display_type: DisplayType::Indicator,
                outputs: &["mfi"],
            },
            DisplayGroup {
                offset: None,
                id: "typprice",
                label: "Typical Price",
                display_type: DisplayType::Overlay,
                outputs: &["typprice"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        let [high, low, close, volume] = *inputs;
        validate_inputs(inputs, Self::min_data(options))?;
        let (mut mfi_line, mut typprice_line) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    typprice_line: len
                ),
            )
        };
        let (mut state, inputs, typprice) = {
            let period = options[0] as usize;
            let offset = crate::slice_outputs_start!(mfi_line.len(), typprice_line);
            let state = State::init_state((high, low, close, volume), period, &mut typprice_line);
            (
                state,
                (
                    &high[period..],
                    &low[period..],
                    &close[period..],
                    &volume[period..],
                ),
                &mut typprice_line[offset..],
            )
        };

        // Perform the main MFI calculation
        cycle_mfi(inputs, &mut state, &mut mfi_line, typprice);

        Ok((vec![mfi_line, typprice_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mfi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Mfi {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mfi_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
