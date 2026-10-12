use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::max::State as MaxState;
use crate::indicators::min::State as MinState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(high: &[f64], low: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            state,
            period,
        }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let period = self.period;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let (mut aroon_up_line, mut aroon_down_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        cycle_aroon(
            (&self.high, &self.low),
            period,
            (&mut aroon_down_line, &mut aroon_up_line),
            &mut self.state,
        );

        self.high.drain(..self.high.len() - period);
        self.low.drain(..self.low.len() - period);

        Ok(vec![aroon_down_line, aroon_up_line])
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
    pub multiplier: f64,
}
impl State<Cold> {
    pub fn new(min: f64, min_trail: usize, max: f64, max_trail: usize, period: usize) -> Self {
        State {
            min_state: MinState::new(min, min_trail),
            max_state: MaxState::new(max, max_trail),
            multiplier: multiplier(period),
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> State<Warm> {
        let multiplier = multiplier(period);
        let min_state = MinState::init_state(low, period);
        let max_state = MaxState::init_state(high, period);

        State {
            min_state,
            max_state,
            multiplier,
        }
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (high, low, i, period): (&[f64], &[f64], usize, usize),
    ) -> (f64, f64) {
        let (_, min_trail) =
            self.min_state
                .calc_chuncked_unchecked::<N>((low, i, (period, period)));
        let (_, max_trail) =
            self.max_state
                .calc_chuncked_unchecked::<N>((high, i, (period, period)));

        calc_aroon(min_trail, max_trail, period, self.multiplier)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], usize, usize);
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, i, period): Self::Inputs<'a>) -> Self::Outputs {
        let (_, min_trail) = self.min_state.calc((low, i, (period, period)));
        let (_, max_trail) = self.max_state.calc((high, i, (period, period)));

        calc_aroon(min_trail, max_trail, period, self.multiplier)
    }
    #[inline(always)]
    unsafe fn calc_unchecked(&mut self, inputs: Self::Inputs<'_>) -> Self::Outputs {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
#[inline(always)]
fn calc_aroon(min_trail: usize, max_trail: usize, period: usize, multiplier: f64) -> (f64, f64) {
    let aroon_up = (period - max_trail) as f64 * multiplier;
    let aroon_down = (period - min_trail) as f64 * multiplier;
    (aroon_down, aroon_up)
}
/// Performs the main calculation loop for the Aroon indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high and low price slices.
/// * `period` - The period for the Aroon calculation.
/// * `multiplier` - The multiplier used to scale Aroon values (100 / period).
/// * `output_lines` - A tuple of mutable slices for storing the Aroon down and Aroon up lines.
/// * `state` - A mutable reference to the current indicator state.
fn cycle_aroon(
    inputs: (&[f64], &[f64]),
    period: usize,
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State<Warm>,
) {
    let (high, low) = inputs;
    let (aroon_down_line, aroon_up_line) = output_lines;
    for (j, i) in (period..high.len()).enumerate() {
        unsafe {
            (
                *aroon_down_line.get_unchecked_mut(j),
                *aroon_up_line.get_unchecked_mut(j),
            ) = state.calc_unchecked((high, low, i, period));
        }
    }
    //println!("Regular SEARCH COUNT: {:?}, period: {:?}", count, period);
}

pub fn multiplier(period: usize) -> f64 {
    100.0 / period as f64
}

pub struct Aroon;

impl Indicator<INPUTS, OPTIONS> for Aroon {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "aroon",
        full_name: "Aroon",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["aroon_down", "aroon_up"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "aroon",
            label: "AROON",
            display_type: DisplayType::Indicator,
            outputs: &["aroon_down", "aroon_up"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let period = options[0] as usize;
        let high = inputs[0];
        let low = inputs[1];

        let (mut aroon_up_line, mut aroon_down_line) = {
            let capacity = Self::output_length(high.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let mut state = State::init_state(high, low, period);
        cycle_aroon(
            (high, low),
            period,
            (&mut aroon_down_line, &mut aroon_up_line),
            &mut state,
        );

        Ok((
            vec![aroon_down_line, aroon_up_line],
            IndicatorState::new(high, low, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::aroon_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Aroon {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::aroon_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Aroon {
    /// The graph is leaf data — `crate::cse::recipes::aroon` (the two
    /// `(p, p)`-window rolls, their trails, and the trail-scaled lines;
    /// publish leads `period`). `aroon_graph` lives in the leaf and is
    /// shared byte-identically by `aroonosc`.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::aroon(options)
    }
}

/// Rebuild the hand `aroon::State<Warm>` from the two roll chains. The
/// `multiplier` is recomputed from the period (derivable, never stored — same
/// convention as the `Ema` restorers). The `high`/`low` tails the hand
/// `IndicatorState` keeps are the engine's host anchor window (`period` bars
/// each, since `look_back = period`).
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    match restore_lines(&v) {
        Some(state) => crate::cse::IndicatorStateBox::Aroon(state),
        None => crate::cse::IndicatorStateBox::Other,
    }
}

/// The two roll chains → the hand aroon `State<Warm>`. Shared with
/// `aroonosc::restore_state`, whose state IS a wrapped aroon state.
pub(crate) fn restore_lines(v: &[crate::cse::ChainState]) -> Option<State<Warm>> {
    let [crate::cse::ChainState::RollMin {
        value: min,
        trail: min_t,
        period,
        ..
    }, crate::cse::ChainState::RollMax {
        value: max,
        trail: max_t,
        ..
    }] = v
    else {
        return None;
    };
    Some(State {
        min_state: MinState::new(*min, *min_t as usize).into_warm(),
        max_state: MaxState::new(*max, *max_t as usize).into_warm(),
        multiplier: multiplier(*period as usize),
    })
}
