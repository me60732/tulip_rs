use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    /// Creates a new `IndicatorState` for streaming continuation.
    ///
    /// # Arguments
    ///
    /// * `real` - The full real price slice from the just-completed batch.
    /// * `sum` - The current rolling sum of absolute price changes (carried forward).
    /// * `period` - The EF lookback period.
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            period,
            state,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let mut ef_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };

        cycle_ef(&self.real, &mut self.state, self.period, &mut ef_line);
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![ef_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub sum: f64,
    pub prev: f64,
    pub drop: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(sum: f64, prev: f64, drop: f64) -> Self {
        Self {
            sum,
            prev,
            drop,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            sum: self.sum,
            prev: self.prev,
            drop: self.drop,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize, ef_line: &mut [f64]) -> State<Warm> {
        let sum = (1..=period).map(|i| (real[i] - real[i - 1]).abs()).sum();

        let (value, last_value) = (real[period], real[0]);

        let ef = if sum != 0.0 {
            (value - last_value).abs() / sum
        } else {
            0.0
        };
        //crate::init_store_optional_outputs!(period, real.len(), ef_line => ef);
        ef_line[0] = ef;

        State {
            sum,
            prev: value,
            drop: last_value,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (value, last_value): Self::Inputs<'a>) -> f64 {
        self.sum += (value - self.prev).abs() - (last_value - self.drop).abs();
        self.prev = value;
        self.drop = last_value;

        if self.sum != 0.0 {
            (value - last_value).abs() / self.sum
        } else {
            0.0
        }
    }
}
/// Performs the main calculation loop for the EF indicator.
///
/// # Arguments
///
/// * `real` - Full price slice including the lookback window at the front.
///   The loop iterates `(period+1)..real.len()`, so the number of outputs written
///   equals `real.len() - period - 1`; `ef_line` must be at least that long.
/// * `sum` - Mutable reference to the rolling absolute-movement accumulator.
/// * `period` - The EF lookback period.
/// * `ef_line` - Output buffer; receives one value per loop iteration (written
///   starting at index 0).
fn cycle_ef(real: &[f64], state: &mut State<Warm>, period: usize, ef_line: &mut [f64]) {
    //real.iter().enumerate().skip(start).for_each(|(i, value)| {
    for (j, i) in (period..real.len()).enumerate() {
        let inputs = unsafe { (*real.get_unchecked(i), *real.get_unchecked(j)) };
        let ef = state.calc(inputs);

        unsafe { *ef_line.get_unchecked_mut(j) = ef };
    }
}

pub struct Ef;
impl Indicator<INPUTS, OPTIONS> for Ef {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ef",
        indicator_type: IndicatorType::Trend,
        full_name: "Efficiency Ratio",
        inputs: &["real"],
        options: &["period"],
        outputs: &["ef"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "ef",
            label: "EF",
            display_type: DisplayType::Indicator,
            outputs: &["ef"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut ef_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(real, period, &mut ef_line);
        cycle_ef(&real[1..], &mut state, period, &mut ef_line[1..]);

        Ok((vec![ef_line], IndicatorState::new(real, state, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ef_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Ef {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ef_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
