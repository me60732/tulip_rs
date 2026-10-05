use std::f64;

use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::max::State as MaxState;
use crate::indicators::medprice::calc as calc_medprice;
use crate::indicators::min::State as MinState;
use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, period: usize) -> Self {
        Self { state, period }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut fisher_line, mut signal_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let [high, low] = inputs;

        cycle_fisher(
            (high, low),
            self.period,
            (&mut fisher_line, &mut signal_line),
            &mut self.state,
        );

        Ok(vec![fisher_line, signal_line])
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: MirrorBuffer<S>,
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
    pub val1: f64,
    pub fish: f64,
}

impl State<Cold> {
    pub fn new(high: f64, low: f64, period: usize) -> Self {
        let medprice = calc_medprice(high, low);
        let mut buffer = MirrorBuffer::new(period);
        buffer.push(medprice);
        State {
            buffer,
            min_state: MinState::new(medprice, period),
            max_state: MaxState::new(medprice, period),
            val1: 0.0,
            fish: 0.0,
        }
    }
    pub fn into_full(self) -> State<Warm> {
        State {
            buffer: self.buffer.into_full(),
            min_state: self.min_state.into_warm(),
            max_state: self.max_state.into_warm(),
            val1: self.val1,
            fish: self.fish,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        fisher_line: &mut [f64],
        signal_line: &mut [f64],
    ) -> State<Warm> {
        let mut state = Self::new(high[0], low[0], period);
        let mut i = 1;
        while !state.buffer.is_full() {
            let medprice = calc_medprice(high[i], low[i]);
            state.buffer.push(medprice);
            let (min, _) = state.buffer.min(&mut state.min_state, medprice);
            let (max, _) = state.buffer.max(&mut state.max_state, medprice);
            if i == period - 1 {
                (fisher_line[0], signal_line[0]) = state.calc_fisher(min, max, medprice);
            }
            i += 1;
        }

        state.into_full()
    }
}
impl<S> State<S> {
    #[inline(always)]
    fn calc_fisher(&mut self, min: f64, max: f64, medprice: f64) -> (f64, f64) {
        // Correctly named constants
        const PRICE_WEIGHT: f64 = 0.66; // 0.33 * 2.0 - weight for new normalized price
        const SMOOTH_WEIGHT: f64 = 0.67; // smoothing factor for exponential average
        const MIN_MM: f64 = 0.001;

        let mut val1 = self.val1;
        let mm = (max - min).max(MIN_MM);

        // Use mul_add for better precision
        val1 = PRICE_WEIGHT.mul_add((medprice - min) / mm - 0.5, SMOOTH_WEIGHT * val1);

        // Clamp val1 to the range [-0.999, 0.999]
        if val1 > 0.99 {
            val1 = 0.999;
        } else if val1 < -0.99 {
            val1 = -0.999;
        }
        self.val1 = val1;

        let signal = self.fish;

        let ln_arg = (1.0 + val1) / (1.0 - val1);

        self.fish = 0.5 * (ln_arg.ln() + signal); //state.fish);
        (self.fish, signal)
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, usize);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc(&mut self, (high, low, period): Self::Inputs<'_>) -> Self::Outputs {
        let medprice = calc_medprice(high, low);
        self.buffer.push(medprice);
        let (min, _) = self.buffer.min(&mut self.min_state, medprice, period);
        let (max, _) = self.buffer.max(&mut self.max_state, medprice, period);
        self.calc_fisher(min, max, medprice)
    }
}

/// Performs the main calculation loop for the Fisher Transform indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple containing high and low price slices.
/// * `period` - The period for the Fisher Transform calculation.
/// * `output_lines` - A tuple containing mutable references to fisher and signal vectors.
/// * `state` - A mutable reference to the indicator state.
fn cycle_fisher(
    inputs: (&[f64], &[f64]),
    period: usize,
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State<Warm>,
) {
    let (fisher_line, signal_line) = output_lines;
    let (high, low) = inputs;
    for i in 0..high.len() {
        let (h, l) = unsafe { (*high.get_unchecked(i), *low.get_unchecked(i)) };
        let (fisher, signal) = state.calc((h, l, period));
        unsafe {
            *fisher_line.get_unchecked_mut(i) = fisher;
            *signal_line.get_unchecked_mut(i) = signal;
        }
    }
}

pub struct Fisher;
impl Indicator<INPUTS, OPTIONS> for Fisher {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "fisher",
        full_name: "Fisher Transform",
        indicator_type: IndicatorType::Momentum,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["fisher", "fisher_signal"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "fisher",
            label: "FISHER",
            display_type: DisplayType::Indicator,
            outputs: &["fisher", "fisher_signal"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let period = options[0] as usize;
        let [high, low] = inputs;

        let (mut fisher_line, mut signal_line) = {
            let capacity = Self::output_length(high.len(), options);
            (vec![0.0; capacity], vec![0.0; capacity])
        };

        let mut state = State::init_state(high, low, period, &mut fisher_line, &mut signal_line);

        let outputs = (&mut fisher_line[1..], &mut signal_line[1..]);
        let inputs = (&high[period..], &low[period..]);

        cycle_fisher(inputs, period, outputs, &mut state);

        Ok((
            vec![fisher_line, signal_line],
            IndicatorState { state, period },
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::fisher_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Fisher {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::fisher_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

/// The canonical node graph for `fisher`, mirroring [`State::calc_fisher`] op for
/// op: `med = 0.5·(high+low)`; a `(period, period−1)` min/max over the DERIVED
/// `med` (the §6.36 cold seed makes the first window include bar 0); `inner =
/// (med−min)/max(max−min,1e-3) − 0.5`; `val1` with the clamp folded IN
/// ([`Node::IirClamped`]); `fish = 0.5·(ln((1+val1)/(1−val1)) + prev_fish)`
/// ([`Node::Iir`]); `signal = prev_fish` ([`Node::Prev`]). Chains idle below
/// `feed = lead(src)`, matching the hand's first `calc_fisher` at bar `period−1`.
/// 1e-12 relative vs the hand scalar (`Node::Ln` = `ln_unchecked`); byte-exact
/// cold-vs-resume.
impl crate::cse::Planable for Fisher {
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        use crate::cse::{Node, Recipe, Sink};
        let period = options[0] as u32;
        let mut r = Recipe::new();

        let hi = r.add(Node::Input { stream: 0 });
        let lo = r.add(Node::Input { stream: 1 });

        // med = 0.5 · (high + low), the hand's `medprice::calc`.
        let sum = r.add(Node::Add { a: hi, b: lo });
        let half = r.add(Node::Const {
            bits: 0.5f64.to_bits(),
        });
        let med = r.add(Node::Mul { a: half, b: sum });

        // window min/max over the derived `med` (`look_back = period − 1`).
        let mn = r.add(Node::RollMin {
            src: med,
            period,
            look_back: period - 1,
        });
        let mx = r.add(Node::RollMax {
            src: med,
            period,
            look_back: period - 1,
        });

        // mm = max(mx − mn, 1e-3); inner = (med − mn) / mm − 0.5.
        let eps = r.add(Node::Const {
            bits: 0.001f64.to_bits(),
        });
        let range = r.add(Node::Sub { a: mx, b: mn });
        let mm = r.add(Node::Max { a: range, b: eps });
        let diff = r.add(Node::Sub { a: med, b: mn });
        let ratio = r.add(Node::Div { a: diff, b: mm });
        let inner = r.add(Node::Sub { a: ratio, b: half });

        // clamp thresholds/values as `Const` children, before the node that uses
        // them. val1 = 0.66·inner + 0.67·prev, then the hand's if/elif clamp.
        let hi_t = r.add(Node::Const {
            bits: 0.99f64.to_bits(),
        });
        let hi_v = r.add(Node::Const {
            bits: 0.999f64.to_bits(),
        });
        let lo_t = r.add(Node::Const {
            bits: (-0.99f64).to_bits(),
        });
        let lo_v = r.add(Node::Const {
            bits: (-0.999f64).to_bits(),
        });
        let val1 = r.add(Node::IirClamped {
            src: inner,
            a_bits: 0.66f64.to_bits(),
            b_bits: 0.67f64.to_bits(),
            hi_t,
            hi_v,
            lo_t,
            lo_v,
        });

        // fish = 0.5·(ln((1+val1)/(1−val1)) + prev_fish); signal = prev_fish.
        let one = r.add(Node::Const {
            bits: 1.0f64.to_bits(),
        });
        let numer = r.add(Node::Add { a: one, b: val1 });
        let denom = r.add(Node::Sub { a: one, b: val1 });
        let ln_arg = r.add(Node::Div { a: numer, b: denom });
        let ln = r.add(Node::Ln { src: ln_arg });
        let fish = r.add(Node::Iir {
            src: ln,
            a_bits: 0.5f64.to_bits(),
            b_bits: 0.5f64.to_bits(),
        });
        let signal = r.add(Node::Prev { src: fish });

        r.kind = "fisher";
        r.options = vec![period as f64];
        r.outputs = vec![Sink::iface(fish), Sink::iface(signal)];
        r
    }
}
