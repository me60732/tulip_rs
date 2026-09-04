use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::sma::{multiplier as sma_multiplier, State as SmaState};
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
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            state,
            period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut wma_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_wma(
            &self.real,
            &mut self.state,
            self.period,
            (&mut wma_line, &mut sma_line),
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![wma_line, sma_line])
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub sma_state: SmaState<S>,
    pub weighted_sum: f64,
    pub(crate) period: f64,
    pub(crate) weights: f64,
}
impl State<Cold> {
    pub fn new(sum: f64, weighted_sum: f64, period: usize) -> Self {
        let (_, weights, n) = multiplier(period);
        Self {
            sma_state: SmaState::new(sum, period),
            weighted_sum,
            weights,
            period: n,
        }
    }
    pub fn init_state(prev_real: &[f64], period: usize) -> State<Warm> {
        let mut sum: f64 = 0.0;
        let mut weighted_sum: f64 = 0.0;

        for (i, &value) in prev_real.iter().take(period).enumerate() {
            sum += value;
            weighted_sum += value * (i + 1) as f64;
        }
        let (_, weights, n) = multiplier(period);
        State {
            sma_state: SmaState::new(sum, period).into_warm(),
            weighted_sum,
            weights,
            period: n,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value): Self::Inputs<'a>) -> Self::Outputs {
        self.weighted_sum -= self.sma_state.sum;

        let sma = self.sma_state.calc((value, prev_value));

        self.weighted_sum += value * self.period;

        let wma = self.weighted_sum * self.weights;

        (wma, sma)
    }
}

/// Performs the main calculation loop for the WMA indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - Mutable reference to the rolling `State` (sum and weighted sum).
/// * `period` - The period for the WMA calculation.
/// * `multipliers` - A tuple of `(sma_multiplier, weights, n)` from `multiplier()`.
/// * `out_vecs` - Mutable output slices: `(wma_line, sma_line)`.
fn cycle_wma(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (wma_line, sma_line) = out_vecs;
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (wma, sma);
        unsafe {
            (wma, sma) = state.calc((*real.get_unchecked(i), *real.get_unchecked(j)));
            *wma_line.get_unchecked_mut(j) = wma;
        }
        crate::store_optional_outputs!(j,
            want_sma, sma_line => sma
        );
    }
}

/*#[inline(always)]
pub fn multiplier(period: usize) -> (f64, f64, f64) {
    let n = period as f64;
    let weights = n * (n + 1.0) / 2.0;
    (sma_multiplier(period), weights, n)
}*/
pub fn multiplier(period: usize) -> (f64, f64, f64) {
    let n = period as f64;
    let weights_recip = 2.0 / (n * (n + 1.0)); // reciprocal, computed once
    (sma_multiplier(period), weights_recip, n)
}

pub struct Wma;
impl Indicator<INPUTS, OPTIONS> for Wma {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "wma",
        indicator_type: IndicatorType::Trend,
        full_name: "Weighted Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["wma"],
        optional_outputs: &["sma"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "wma",
            label: "Moving Averages",
            display_type: DisplayType::Overlay,
            outputs: &["wma", "sma"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let (mut wma_line, mut sma_line) = {
            let capacity = Self::output_length(real.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        let mut state = State::init_state(real, period);

        cycle_wma(real, &mut state, period, (&mut wma_line, &mut sma_line));

        Ok((
            vec![wma_line, sma_line],
            IndicatorState::new(real, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wma_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Wma {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wma_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
