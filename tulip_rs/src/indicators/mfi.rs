use crate::common::{validate_inputs, validate_options};
use crate::cse::{Planable, Recipe};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
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
#[derive(Serialize, Deserialize, Clone)]
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

        //let pos_flow = if price_change > 0.0 { money_flow } else { 0.0 };   // single cmov
        //let neg_flow = if price_change < 0.0 { money_flow } else { 0.0 };    // independent cmov
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

        let total = self.pos_sum + self.neg_sum;
        if total < 1e-14 {
            0.0
        } else {
            self.pos_sum / total * 100.0
        }
        //self.pos_sum / (self.pos_sum + self.neg_sum).max(f64::EPSILON) * 100.0
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

impl Planable for Mfi {
    fn recipe(options: &[f64]) -> Recipe {
        crate::cse::recipes::mfi(options)
    }
}

impl crate::cse::Restorable<INPUTS, OPTIONS> for Mfi {
    const KIND: &'static str = "mfi";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        // The two sums are `Node::RollSum` → `RollSumRing`, not `Sma` (the
        // `RollAge` head is the signed-volume age roll, unchanged). Those two
        // sums ARE this state's accumulators (`pos_sum`, `neg_sum`), so they are
        // carried rather than re-summed.
        let [crate::cse::ChainState::RollAge { .. }, crate::cse::ChainState::RollSumRing {
            sum: pos_sum,
            period,
            ..
        }, crate::cse::ChainState::RollSumRing { sum: neg_sum, .. }] = v.as_slice()
        else {
            return None;
        };
        if windows.len() < 4 {
            return None;
        }

        let period = *period as usize;
        // `init_state` is the WRONG tool here, and the reason is arithmetic, not
        // style: it runs exactly `period` steps starting at slice index 0, so the
        // pairs it writes land on bars `seam-period .. seam-1` — one bar short of
        // the seam — and its very first step compares a typ price against itself,
        // forcing a zero flow the real history never had. Slicing to `period + 1`
        // fixed the seam (300/300 wrong → 192/300 wrong from row 5) but could not
        // fix either of those, because they are the loop's shape.
        //
        // So the buffer is BUILT, not re-seeded: the last `period` money-flow
        // pairs, each from `tp[i]` versus `tp[i-1]` over the last `period + 2`
        // bars. That is exact because `(tp[i] - tp[i-1])` and `tp[i] * volume[i]`
        // are pointwise in the raw columns — the same values the hand pushed, in
        // the order it pushed them. The two accumulators and the seam typ price
        // then come from the chains and the last bar, never from a re-sum.
        let need = period + 2;
        if period == 0
            || windows[0].len() < need
            || windows[1].len() < need
            || windows[2].len() < need
            || windows[3].len() < need
        {
            return None;
        }
        let high = &windows[0][windows[0].len() - need..];
        let low = &windows[1][windows[1].len() - need..];
        let close = &windows[2][windows[2].len() - need..];
        let volume = &windows[3][windows[3].len() - need..];

        // `MultiBuffer`'s first parameter is the LANE COUNT (a const), so it is
        // written `::<2, f64, Cold>` — the state's buffer is exactly the two
        // money-flow lanes `[pos_flow, neg_flow]`.
        let mut buffer = Buffer::<2, f64, Cold>::new(period);
        for i in 1..need {
            let prev_typprice = calc_typprice(high[i - 1], low[i - 1], close[i - 1]);
            let typprice = calc_typprice(high[i], low[i], close[i]);
            let flow = typprice * volume[i];
            let change = typprice - prev_typprice;
            let pair = if change > 0.0 {
                [flow, 0.0]
            } else if change < 0.0 {
                [0.0, flow]
            } else {
                [0.0, 0.0]
            };
            buffer.push(pair);
        }
        let typprice = calc_typprice(high[need - 1], low[need - 1], close[need - 1]);

        Some(State {
            buffer: buffer.into_full(),
            typprice,
            pos_sum: *pos_sum,
            neg_sum: *neg_sum,
        })
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
