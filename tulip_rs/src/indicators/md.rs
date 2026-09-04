use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::sma::State as SmaState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

use std::simd::{num::SimdFloat, Simd};

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub SmaState<S>);
impl<S> Deref for State<S> {
    type Target = SmaState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl State<Cold> {
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        State(SmaState::init_state(real, period))
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State(self.0.into_warm())
    }
    pub fn new(sum: f64, period: usize) -> Self {
        State(SmaState::new(sum, period))
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, &'a [f64]);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value, slice): Self::Inputs<'a>) -> (f64, f64) {
        let sma = self.0.calc((value, prev_value));

        let mean_deviation = calc_md_simd::<4>(slice, sma, self.multiplier);
        (mean_deviation, sma)
    }
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
        // Calculate capacities
        self.real.extend_from_slice(inputs[0]);

        let (mut md_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        cycle_md(
            &self.real,
            &mut self.state,
            self.period,
            &mut md_line,
            &mut sma_line,
        );

        self.real.drain(..self.real.len() - self.period);
        Ok(vec![md_line, sma_line])
    }
}

/// Performs the main calculation loop for the MD indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `sum` - The running sum for the SMA calculation.
/// * `period` - The period for the MD calculation.
/// * `multiplier` - The SMA multiplier (`1.0 / period`).
/// * `md_line` - A mutable slice for storing the MD output values.
/// * `sma_line` - A mutable slice for storing optional SMA output values.
///
/// # Returns
///
/// The updated running sum.
fn cycle_md(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    md_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (want_sma, _) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let inputs = unsafe {
            (
                *real.get_unchecked(i),
                *real.get_unchecked(j),
                real.get_unchecked(j + 1..=i),
            )
        };

        let (md, sma) = state.calc(inputs);
        unsafe { *md_line.get_unchecked_mut(j) = md };

        if want_sma {
            crate::store_optional_outputs!(j,
                want_sma, sma_line => sma
            );
        }
    }
}

pub struct Md;

impl Indicator<INPUTS, OPTIONS> for Md {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "md",
        indicator_type: IndicatorType::Volatility,
        full_name: "Mean Deviation",
        inputs: &["real"],
        options: &["period"],
        outputs: &["md"],
        optional_outputs: &["sma"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "md",
            label: "MD",
            display_type: DisplayType::Indicator,
            outputs: &["md"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        if options[0] < 1.0 {
            return Err(IndicatorError::InvalidOptions);
        }
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut state = State::init_state(real, period);
        let (mut md_line, mut sma_line) = {
            let capacity = Self::output_length(real.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        cycle_md(real, &mut state, period, &mut md_line, &mut sma_line);

        Ok((
            vec![md_line, sma_line],
            IndicatorState::new(real, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::md_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Md {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::md_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[inline(always)]
pub(crate) fn calc_md_simd<const N: usize>(slice: &[f64], sma: f64, multiplier: f64) -> f64 {
    //let mut abs_dev_sum = 0.0;
    let sma_vec = Simd::<f64, N>::splat(sma);

    let mut sum = Simd::splat(0.0);
    for chunk in slice.chunks_exact(N) {
        let vals = Simd::from_slice(chunk);
        sum += (vals - sma_vec).abs();
    }

    let mut abs_dev_sum = sum.reduce_sum();
    // Handle remainder
    let processed_len = (slice.len() / N) * N;
    let remainder = &slice[processed_len..];
    abs_dev_sum += remainder.iter().map(|&x| (x - sma).abs()).sum::<f64>();

    abs_dev_sum * multiplier
}
#[inline(always)]
pub fn calc_md(real: &[f64], sma: f64, multiplier: f64) -> f64 {
    real.iter().map(|&x| (x - sma).abs()).sum::<f64>() * multiplier
}
