use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

pub type IndicatorState = State;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub prev_close: f64,
}
impl TState for State {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let hc = (high - self.prev_close).abs();
        let lc = (low - self.prev_close).abs();
        self.prev_close = close;

        let mut tr = high - low;
        if hc > tr {
            tr = hc;
        }
        if lc > tr {
            tr = lc;
        }

        tr
    }
}
impl State {
    pub fn new(prev_close: f64) -> Self {
        Self { prev_close }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let mut tr_line = crate::uninit_vec!(f64, high.len());

        cycle_tr(high, low, close, self, &mut tr_line);
        Ok(vec![tr_line])
    }
}

/// Performs the main calculation loop for the TR indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `prev_close` - The previous close price.
/// * `start` - The starting index into `high`, `low`, and `close` to begin reading from.
/// * `tr_line` - A mutable slice for storing the TR output values.
#[inline(always)]
fn cycle_tr(high: &[f64], low: &[f64], close: &[f64], state: &mut State, tr_line: &mut [f64]) {
    for i in 0..high.len() {
        unsafe {
            *tr_line.get_unchecked_mut(i) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            ));
        }
    }
}

pub struct Tr;

impl Indicator<INPUTS, OPTIONS> for Tr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "tr",
        full_name: "True Range",
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["tr"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["tr"],
        }],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2
    }

    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len - 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_inputs(inputs, Self::min_data(_options))?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let mut tr_line = {
            let capacity = Self::output_length(high.len(), _options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::new(close[0]);
        cycle_tr(&high[1..], &low[1..], &close[1..], &mut state, &mut tr_line);

        Ok((vec![tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::tr_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
