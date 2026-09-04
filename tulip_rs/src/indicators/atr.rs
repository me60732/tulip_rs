use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::tr::{State as TrState, Tr};
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
    pub wilders_state: WildersState<S>,
    pub tr_state: TrState,
}

impl State<Cold> {
    pub fn new(wilders_state: WildersState, tr_state: TrState) -> Self {
        Self {
            wilders_state,
            tr_state,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
        composite: bool,
    ) -> State<Warm> {
        let mut atr = high[0] - low[0];
        let mut tr_state = TrState::new(close[0]);
        if period < high.len() {
            for (i, ((&h, &l), &c)) in high
                .iter()
                .zip(low.iter())
                .zip(close)
                .enumerate()
                .take(period)
                .skip(1)
            {
                let tr = tr_state.calc((h, l, c));
                atr += tr;
                if tr_line.len() > 0 {
                    tr_line[i - 1] = tr;
                }
            }
        }
        if !composite {
            atr /= period as f64;
        }
        State {
            wilders_state: WildersState::new(atr, period).into_warm(),
            tr_state: TrState::new(close[period - 1]),
        }
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn partial_calc(&mut self, inputs: (f64, f64, f64)) -> (f64, f64) {
        let tr = self.tr_state.calc(inputs);
        let atr = self.wilders_state.partial_calc(tr);
        (atr, tr)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let tr = self.tr_state.calc(inputs);
        let atr = self.wilders_state.calc(tr);
        (atr, tr)
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut atr_line = crate::uninit_vec!(f64, inputs[0].len());

        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: inputs[0].len()
        );
        cycle_atr(
            (inputs[0], inputs[1], inputs[2]),
            self,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![atr_line, tr_line])
    }
}

/// Performs the main calculation loop for the ATR indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high, low, and close price slices.
/// * `state` - A mutable reference to the current ATR state.
/// * `outputs` - A tuple of mutable slices for storing the ATR line and optional TR line.
fn cycle_atr(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    outputs: (&mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (atr_line, tr_line) = outputs;
    let (_, want_tr) = crate::calc_want_flags!(tr_line);

    for i in 0..high.len() {
        let (atr, tr);
        unsafe {
            let inputs = (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
            (atr, tr) = state.calc(inputs);
            *atr_line.get_unchecked_mut(i) = atr;
        }
        crate::store_optional_outputs!(i,
            want_tr, tr_line => tr
        );
    }
}

pub struct Atr;
impl Indicator<INPUTS, OPTIONS> for Atr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "atr",
        full_name: "Average True Range",
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["atr"],
        optional_outputs: &["tr"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        }],
    };

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

        let mut atr_line = {
            let atr_capacity = Self::output_length(high.len(), options);
            crate::uninit_vec!(f64, atr_capacity)
        };
        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: Tr::output_length(high.len(), &[])
        );
        let mut state = State::init_state(high, low, close, period, &mut tr_line, false);
        let tr_offset = crate::slice_outputs_start!(atr_line.len(), tr_line);
        cycle_atr(
            (&high[period..], &low[period..], &close[period..]),
            &mut state,
            (&mut atr_line, &mut tr_line[tr_offset..]),
        );

        Ok((vec![atr_line, tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::atr_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Atr {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::atr_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
