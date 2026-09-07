use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::ema::{multiplier, Ema};
use crate::indicators::{
    ema::State as EmaState,
    tr::{State as TrState, Tr},
};
pub use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// `S = NotFull` during warmup, `S = Full` once the buffer is filled.
/// `#[serde(bound = "")]` suppresses the auto-derived `S: Serde` bound — the
/// buffer's own Serde impl is already generic over any `S`.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub ema_state: EmaState<S>,
    pub tr_state: TrState,
}

impl State<Cold> {
    pub fn new(prev_close: f64, ema: f64, period: usize) -> Self {
        Self {
            tr_state: TrState::new(prev_close),
            buffer: Buffer::new(period),
            ema_state: EmaState::new(ema, period),
        }
    }

    pub fn init_state(
        [high, low, close]: &[&[f64]; INPUTS],
        period: usize,
        tr_line: &mut [f64],
        ema_line: &mut [f64],
    ) -> State<Warm> {
        let mut tr_state = TrState::new(close[0]);
        let mut ema_state = EmaState::new(high[0] - low[0], period).into_warm();
        let mut buffer = Buffer::new(period);
        for i in 1..period * 2 - 1 {
            let inputs = (high[i], low[i], close[i]);
            let (tr, ema);
            tr = tr_state.calc(inputs);
            ema = ema_state.calc(tr);
            buffer.push(ema);

            crate::init_store_optional_outputs!(i, high.len(),
                tr_line => tr,
                ema_line => ema
            );
        }

        // Buffer is now full — transition to the operational typestate.
        State {
            buffer: buffer.into_full(),
            ema_state,
            tr_state,
        }
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let old_ema = self.buffer.front(); // T, not Option<T>
        let tr = self.tr_state.calc(inputs);
        let ema = self.ema_state.calc(tr);
        self.buffer.push(ema);

        ((ema - old_ema) / old_ema * 100.0, tr, ema)
    }
}

pub type IndicatorState = State<Warm>;

impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut trvi_line, (mut tr_line, mut ema_line)) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, inputs[0].len()),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: len,
                    ema_line: len
                ),
            )
        };
        let [high, low, close] = inputs;
        cycle(
            (high, low, close),
            self,
            &mut trvi_line,
            (&mut tr_line, &mut ema_line),
        );

        Ok(vec![trvi_line, tr_line, ema_line])
    }
}

/// Performs the main calculation loop for the TRVI indicator.
fn cycle(
    (high, low, close): (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    trvi_line: &mut [f64],
    (tr_line, ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_tr, want_ema) = crate::calc_want_flags!(tr_line, ema_line);

    for i in 0..high.len() {
        // Only the slice accesses are unsafe — calc itself is safe on State<Full>.
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (trvi, tr, ema) = state.calc(inputs);
        unsafe {
            *trvi_line.get_unchecked_mut(i) = trvi;
        }

        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr,
                want_ema, ema_line => ema
            );
        }
    }
}

pub struct Trvi;

impl Indicator<INPUTS, OPTIONS> for Trvi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "trvi",
        indicator_type: IndicatorType::Volatility,
        full_name: "True Range Volatility Indicator",
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["trvi"],
        optional_outputs: &["tr", "ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "trvi",
                label: "True Range Volatility Indicator",
                display_type: DisplayType::Indicator,
                outputs: &["trvi"],
            },
            DisplayGroup {
                offset: None,
                id: "tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["tr", "ema"],
            },
        ],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        (options[0] * 2.0) as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Ema::min_data(options))?;
        let [high, low, close] = *inputs;
        let (mut trvi_line, (mut tr_line, mut ema_line)) = {
            let capacity = Self::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    ema_line: Ema::output_length(tr_capacity, options)
                ),
            )
        };

        let mut state = State::init_state(inputs, period, &mut tr_line, &mut ema_line);

        let (high, low, close) = {
            let from = period * 2 - 1;
            (&high[from..], &low[from..], &close[from..])
        };
        let (tr, ema) = {
            let (tr_offset, ema_offset) =
                crate::slice_outputs_start!(trvi_line.len(), tr_line, ema_line);
            (&mut tr_line[tr_offset..], &mut ema_line[ema_offset..])
        };
        cycle((high, low, close), &mut state, &mut trvi_line, (tr, ema));

        Ok((vec![trvi_line, tr_line, ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trvi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Trvi {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trvi_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
