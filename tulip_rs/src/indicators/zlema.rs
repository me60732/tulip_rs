use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
pub use crate::indicators::ema::multiplier;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    real: Vec<f64>,
    lag: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, lag: usize) -> Self {
        Self {
            state,
            real: real[real.len() - lag..].to_vec(),
            lag,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub zlema: f64,
    pub per: f64,
    pub multiplier: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(real: &[f64], lag: usize, period: usize) -> State<Warm> {
        let (multiplier, per) = multiplier(period);
        State {
            zlema: real[lag - 1],
            multiplier,
            per,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (current, lagged): Self::Inputs<'a>) -> Self::Outputs {
        let adjusted = current + (current - lagged);

        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        self.zlema
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        // Merge stored trailing real values with new input.
        self.real.extend_from_slice(inputs[0]);

        let mut zlema_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_zlema(&self.real, self.lag, &mut self.state, &mut zlema_line);

        self.real.drain(..self.real.len() - self.lag);

        Ok(vec![zlema_line])
    }
}

/// Iterates over the real data slice and computes ZLEMA values for each bar.
///
/// # Arguments
///
/// * `real` - The full input data slice (includes the leading lag values).
/// * `lag` - The number of look-back bars used for zero-lag adjustment.
/// * `state` - Mutable reference to the rolling `State` (previous ZLEMA, multipliers).
/// * `zlema_line` - Mutable output slice for ZLEMA values.
fn cycle_zlema(real: &[f64], lag: usize, state: &mut State<Warm>, zlema_line: &mut [f64]) {
    for (j, i) in (lag..real.len()).enumerate() {
        unsafe {
            *zlema_line.get_unchecked_mut(j) =
                state.calc((*real.get_unchecked(i), *real.get_unchecked(j)))
        };
    }
}

pub struct Zlema;
impl Indicator<INPUTS, OPTIONS> for Zlema {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "zlema",
        full_name: "Zero Lag Exponential Moving Average",
        indicator_type: IndicatorType::Trend,
        // One input: real (can be any price series).
        inputs: &["real"],
        // One option: period.
        options: &["period"],
        outputs: &["zlema"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "zlema",
            label: "ZLEMA",
            display_type: DisplayType::Overlay,
            outputs: &["zlema"],
        }],
    };
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        ((options[0] as usize - 1) / 2) + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let lag = ((period.saturating_sub(1)) / 2).max(1);

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut zlema_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::new(real, lag, period);

        cycle_zlema(real, lag, &mut state, &mut zlema_line);

        Ok((vec![zlema_line], IndicatorState::new(real, state, lag)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::zlema_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Zlema {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::zlema_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
