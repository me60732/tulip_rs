use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::aroon::State as AroonState;
pub use crate::indicators::aroon::OPTIONS;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

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
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let period = self.period;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let capacity = inputs[0].len();
        let mut aroonosc_line = crate::uninit_vec!(f64, capacity);

        let (mut aroon_up_line, mut aroon_down_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            aroon_up_line: capacity,
            aroon_down_line: capacity
        );

        cycle(
            (&self.high, &self.low),
            period,
            &mut aroonosc_line,
            &mut self.state,
            (&mut aroon_down_line, &mut aroon_up_line),
        );

        self.high.drain(..self.high.len() - period);
        self.low.drain(..self.low.len() - period);

        Ok(vec![aroonosc_line, aroon_down_line, aroon_up_line])
    }
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub AroonState<S>);
impl<S> Deref for State<S> {
    type Target = AroonState<S>;
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
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> State<Warm> {
        State(AroonState::init_state(high, low, period))
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], usize, usize);
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (aroon_down, aroon_up) = self.0.calc(inputs);

        (aroon_up - aroon_down, aroon_down, aroon_up)
    }
    #[inline(always)]
    unsafe fn calc_unchecked<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<'a, const N: usize>(
        &mut self,
        inputs: (&'a [f64], &'a [f64], usize, usize),
    ) -> (f64, f64, f64) {
        let (aroon_down, aroon_up) = self.0.calc_chuncked_unchecked::<N>(inputs);

        (aroon_up - aroon_down, aroon_down, aroon_up)
    }
}
/// Performs the main calculation loop for the Aroon Oscillator indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high and low price slices.
/// * `period` - The period for the Aroon Oscillator calculation.
/// * `multiplier` - The multiplier used to scale Aroon values (100 / period).
/// * `aroonosc_line` - A mutable slice for storing the Aroon Oscillator values.
/// * `state` - A mutable reference to the current indicator state.
/// * `out_vecs` - A tuple of mutable slices for storing optional Aroon down and Aroon up lines.
fn cycle(
    inputs: (&[f64], &[f64]),
    period: usize,
    aroonosc_line: &mut [f64],
    state: &mut State<Warm>,
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (high, low) = inputs;

    let (aroon_down_line, aroon_up_line) = out_vecs;
    let (has_optional, want_up, want_down) =
        crate::calc_want_flags!(aroon_up_line, aroon_down_line);

    for (j, i) in (period..high.len()).enumerate() {
        let (aroonosc, aroon_down, aroon_up) = state.calc((high, low, i, period));
        unsafe { *aroonosc_line.get_unchecked_mut(j) = aroonosc };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_up, aroon_up_line => aroon_up,
                want_down, aroon_down_line => aroon_down
            );
        }
    }
}

pub struct AroonOsc;

impl Indicator<INPUTS, OPTIONS> for AroonOsc {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "aroonosc",
        full_name: "Aroon Oscillator",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["aroonosc"],
        optional_outputs: &["aroon_down", "aroon_up"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "aroonosc",
                label: "AROONOSC",
                display_type: DisplayType::Indicator,
                outputs: &["aroonosc"],
            },
            DisplayGroup {
                offset: None,
                id: "aroon_down_aroon_up",
                label: "Aroon",
                display_type: DisplayType::Indicator,
                outputs: &["aroon_down", "aroon_up"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;

        let period = options[0] as usize;
        let high = inputs[0];
        let low = inputs[1];

        let capacity = Self::output_length(high.len(), options);
        //let mut aroonosc_line = vec![0.0; capacity]; //Vec::with_capacity(capacity);
        let mut aroonosc_line = crate::uninit_vec!(f64, capacity);

        let (mut aroon_up_line, mut aroon_down_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            aroon_up_line: capacity,
            aroon_down_line: capacity
        );

        let mut state = State::init_state(high, low, period);
        cycle(
            (&high, &low),
            period,
            &mut aroonosc_line,
            &mut state,
            (&mut aroon_down_line, &mut aroon_up_line),
        );

        Ok((
            vec![aroonosc_line, aroon_down_line, aroon_up_line],
            Self::IndicatorState::new(high, low, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::aroonosc_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for AroonOsc {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::aroonosc_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for AroonOsc {
    /// `AROONOSC` = `aroon_up − aroon_down`, built on
    /// [`aroon::aroon_graph`](super::aroon::aroon_graph) so EVERY node it emits
    /// is byte-identical to aroon's. Co-requested with `Aroon`, the arena
    /// interns the whole shape: one `RollMax` over high, one `RollMin` over
    /// low, both trail projections and both scaled lines are shared, and the
    /// oscillator costs one extra `fsub` + one store — the family's fusion
    /// showcase (the hand runs a second full pass over both series).
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        let period = options[0] as u32;
        let mut r = crate::cse::Recipe::new();
        let g = super::aroon::aroon_graph(&mut r, period);
        let osc = r.add(crate::cse::Node::Sub { a: g.up, b: g.down });
        r.kind = "aroonosc";
        r.options = options.to_vec();
        r.tail_inputs = vec![g.min_roll, g.max_roll];
        r.restorer = Some(restore_state);
        r.outputs = vec![crate::cse::Sink::iface_lead(osc, period)];
        // hand `Info::optional_outputs` order: [aroon_down, aroon_up]; both
        // print from bar `period` (one `cycle` writes all three).
        r.optional_outputs = vec![
            crate::cse::Sink::iface_lead(g.down, period),
            crate::cse::Sink::iface_lead(g.up, period),
        ];
        r
    }
}

/// The osc's hand state is a `#[repr(transparent)]` wrapper over the aroon
/// state, so it restores through the same two roll chains.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    match super::aroon::restore_lines(&v) {
        Some(inner) => crate::cse::IndicatorStateBox::AroonOsc(State(inner)),
        None => crate::cse::IndicatorStateBox::Other,
    }
}
