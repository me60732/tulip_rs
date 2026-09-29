use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

pub use crate::indicators::sma::multiplier;
use crate::indicators::sma::State as SmaState;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        let real = real[real.len() - period..].to_vec();
        Self {
            real,
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

        let (mut stddev_line, mut sma_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        cycle_stddev(
            &self.real,
            &mut self.state,
            self.period,
            &mut stddev_line,
            &mut sma_line,
        );

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![stddev_line, sma_line])
    }
}

impl<S> Deref for State<S> {
    type Target = SmaState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.sma_state
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sma_state
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub sma_state: SmaState<S>,
    pub sum_sq: f64,
}
impl State {
    pub fn new(sum: f64, sum_sq: f64, period: usize) -> Self {
        State {
            sma_state: SmaState::new(sum, period),
            sum_sq,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            sma_state: self.sma_state.into_warm(),
            sum_sq: self.sum_sq,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for i in 0..period {
            sum += real[i];
            sum_sq = real[i].mul_add(real[i], sum_sq);
        }
        State {
            sma_state: SmaState::new(sum, period).into_warm(),
            sum_sq,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value): Self::Inputs<'a>) -> (f64, f64) {
        let sma = self.sma_state.calc((value, prev_value));
        self.sum_sq += value.mul_add(value, -(prev_value * prev_value));
        let mut sd = self.sum_sq.mul_add(self.multiplier, -(sma * sma));
        sd = sd.sqrt().max(f64::EPSILON);

        (sd, sma)
    }
}

/// Performs the main calculation loop for the STDDEV indicator.
///
/// # Arguments
///
/// * `real` - A slice of input values.
/// * `state` - A mutable reference to the current `State` (sum and sum of squares).
/// * `period` - The period for the STDDEV calculation.
/// * `multiplier` - The precomputed multiplier (1/period).
/// * `stddev_line` - A mutable slice for storing the STDDEV output values.
/// * `sma_line` - A mutable slice for storing the optional SMA output values.
fn cycle_stddev(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    stddev_line: &mut [f64],
    sma_line: &mut [f64],
) {
    let (_, want_sma) = crate::calc_want_flags!(sma_line);

    for (j, i) in (period..real.len()).enumerate() {
        let (stddev, sma) = unsafe { state.calc((*real.get_unchecked(i), *real.get_unchecked(j))) };
        unsafe { *stddev_line.get_unchecked_mut(j) = stddev };
        crate::store_optional_outputs!(j,
            want_sma, sma_line => sma
        );
    }
}

pub struct StdDev;

impl Indicator<INPUTS, OPTIONS> for StdDev {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "stddev",
        indicator_type: IndicatorType::Volatility,
        full_name: "Standard Deviation",
        inputs: &["real"],
        options: &["period"],
        outputs: &["stddev"],
        optional_outputs: &["sma"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "stddev",
                label: "STDDEV",
                display_type: DisplayType::Indicator,
                outputs: &["stddev"],
            },
            DisplayGroup {
                offset: None,
                id: "sma",
                label: "SMA",
                display_type: DisplayType::Overlay,
                outputs: &["sma"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let (mut stddev_line, mut sma_line) = {
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

        cycle_stddev(&real, &mut state, period, &mut stddev_line, &mut sma_line);

        Ok((
            vec![stddev_line, sma_line],
            IndicatorState::new(real, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stddev_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for StdDev {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stddev_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for StdDev {
    /// `STDDEV(p)` is one rolling node (sum + sum_sq, hand `calc` op for
    /// op); its optional SMA line is the SAME node the standalone `Sma`
    /// request would build — CSE gives one recurrence, two mirrors.
    /// Lead = `period` (hand reports `len - period` rows, primary and
    /// optional alike).
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        let period = options[0] as u32;
        let mut r = crate::cse::Recipe::new();
        let close = r.add(crate::cse::Node::Input { stream: 0 });
        let sd = r.add(crate::cse::Node::Stddev { src: close, period });
        let m = r.add(crate::cse::Node::Sma { src: close, period });
        r.kind = "stddev";
        r.options = options.to_vec();
        r.tail_inputs = vec![sd];
        r.restorer = Some(restore_state);
        r.outputs.push(crate::cse::Sink::iface_lead(sd, period));
        r.optional_outputs
            .push(crate::cse::Sink::iface_lead(m, period));
        r
    }
}

/// Rebuild the hand `stddev::State<Warm>` from the group sum + this op's
/// `sum_sq` aux line (`get_states`). Like every rolling kind, the price
/// window the `IndicatorState` also holds lives in `JitState.window`.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    let [crate::cse::ChainState::Stddev {
        sum,
        sum_sq,
        period,
    }] = v.as_slice()
    else {
        return crate::cse::IndicatorStateBox::Other;
    };
    crate::cse::IndicatorStateBox::Stddev(State::new(*sum, *sum_sq, *period as usize).into_warm())
}
