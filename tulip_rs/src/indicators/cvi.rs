use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};

pub use crate::indicators::ema::multiplier;
use crate::indicators::ema::State as EmaState;
pub use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub ema_state: EmaState<S>,
}
impl State<Cold> {
    pub fn new(ema: f64, period: usize) -> State<Cold> {
        Self {
            buffer: Buffer::new(period),
            ema_state: EmaState::new(ema, period),
        }
    }
    pub fn init_state([high, low]: &[&[f64]; INPUTS], period: usize) -> State<Warm> {
        use crate::indicators::ema::{calc as calc_ema, multiplier};
        let (multiplier, inv_multiplier) = multiplier(period);
        let mut ema = high[0] - low[0];
        let mut buffer = Buffer::new(period);

        for i in 1..period * 2 - 1 {
            ema = calc_ema(high[i] - low[i], ema, multiplier, inv_multiplier);
            buffer.push(ema);
        }
        State {
            buffer: buffer.into_full(),
            ema_state: EmaState::<Warm> {
                ema,
                inv_multiplier,
                multiplier,
                state: std::marker::PhantomData::<Warm>,
            },
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc(&mut self, (high, low): (f64, f64)) -> f64 {
        let old_ema = self.buffer.front();
        let hl_diff = (high - low).max(f64::EPSILON);
        let ema = self.ema_state.calc(hl_diff);
        self.buffer.push(ema);

        (ema - old_ema) / old_ema * 100.0
    }
}

pub type IndicatorState = State<Warm>;
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut cvi_line = crate::uninit_vec!(f64, inputs[0].len());
        let [high, low] = inputs;
        cycle((high, low), self, &mut cvi_line);

        Ok(vec![cvi_line])
    }
}

/// Performs the main calculation loop for the CVI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `multiplier` - A tuple `(multiplier, inv_multiplier)` derived from the EMA period.
/// * `state` - Mutable reference to the ring buffer holding recent EMA values.
/// * `cvi_line` - Mutable slice to write the CVI output values into.
fn cycle((high, low): (&[f64], &[f64]), state: &mut State<Warm>, cvi_line: &mut [f64]) {
    for i in 0..high.len() {
        unsafe {
            *cvi_line.get_unchecked_mut(i) =
                state.calc((*high.get_unchecked(i), *low.get_unchecked(i)));
        }
    }
}

pub struct Cvi;

impl Indicator<INPUTS, OPTIONS> for Cvi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "cvi",
        indicator_type: IndicatorType::Volatility,
        full_name: "Chaikin Volatility Indicator",
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["cvi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "cvi",
            label: "CVI",
            display_type: DisplayType::Indicator,
            outputs: &["cvi"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        (options[0] * 2.0) as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        let mut cvi_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(&inputs, period);

        let (high, low) = {
            let from = period * 2 - 1;
            (&inputs[0][from..], &inputs[1][from..])
        };
        cycle((high, low), &mut state, &mut cvi_line);

        Ok((vec![cvi_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cvi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Cvi {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cvi_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
