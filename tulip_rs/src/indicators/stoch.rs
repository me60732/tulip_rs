use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

pub use crate::indicators::{max::State as MaxState, min::State as MinState};

use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    state: State<Warm>,
    high: Vec<f64>,
    low: Vec<f64>,
    k_period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, high: &[f64], low: &[f64], k_period: usize) -> Self {
        Self {
            state,
            high: high[high.len() - k_period..].to_vec(),
            low: low[low.len() - k_period..].to_vec(),
            k_period,
        }
    }
}

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let close = inputs[2];

        let (mut k_line, mut d_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        cycle(
            (&self.high, &self.low, close),
            self.k_period,
            0,
            &mut self.state,
            (&mut k_line, &mut d_line),
        );

        self.high.drain(..self.high.len() - self.k_period);
        self.low.drain(..self.low.len() - self.k_period);

        Ok(vec![k_line, d_line])
    }
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub prev_k: Buffer<S>,
    pub prev_d: Buffer<S>,
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
    pub k_sum: f64,
    pub d_sum: f64,
    pub k_multiplier: f64,
    pub d_multiplier: f64,
}

impl State<Cold> {
    pub fn new(min: (f64, usize), max: (f64, usize), k_slow: usize, d_period: usize) -> Self {
        let (k_multiplier, d_multiplier) = multiplier(k_slow, d_period);
        State {
            min_state: MinState::new(min.0, min.1),
            max_state: MaxState::new(max.0, max.1),
            prev_k: Buffer::new(k_slow),
            prev_d: Buffer::new(d_period),
            k_sum: 0.0,
            d_sum: 0.0,
            k_multiplier,
            d_multiplier,
        }
    }

    pub fn init_state(
        inputs: (&[f64], &[f64], &[f64]),
        k_period: usize,
        k_slow: usize,
        d_period: usize,
        k_line: &mut [f64],
    ) -> (State<Warm>, usize, usize) {
        let (high, low, _) = inputs;

        let mut min_state = MinState::init_state(low, k_period + 1);
        let mut max_state = MaxState::init_state(high, k_period + 1);
        let mut prev_k = Buffer::new(k_slow);
        let mut prev_d = Buffer::new(d_period);
        let mut k_sum = 0.0;
        let mut d_sum = 0.0;
        let (k_multiplier, d_multiplier) = multiplier(k_slow, d_period);
        let mut k_count = 0;
        let mut start = 0;
        for i in k_period + 1..k_period + k_slow + d_period {
            let k_fast = calc_kfast::<4>(&mut min_state, &mut max_state, inputs, i, k_period);
            k_sum += k_fast;
            if let Some(k_old) = prev_k.push_with_info(k_fast) {
                k_sum -= k_old;
            }
            if prev_k.is_full() {
                // Buffer was full so a value was replaced.
                let k = k_sum * k_multiplier;
                k_line[k_count] = k;
                k_count += 1;
                d_sum += k;
                prev_d.push(k);
            }
            start = i;
        }
        start += 1;
        (
            State {
                prev_k: prev_k.into_full(),
                prev_d: prev_d.into_full(),
                min_state,
                max_state,
                k_sum,
                d_sum,
                k_multiplier,
                d_multiplier,
            },
            k_count,
            start,
        )
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = ((&'a [f64], &'a [f64], &'a [f64]), usize, usize);
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc(&mut self, inputs: ((&[f64], &[f64], &[f64]), usize, usize)) -> (f64, f64) {
        self.calc_chuncked::<4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn calc_chuncked<const N: usize>(
        &mut self,
        (inputs, i, k_period): ((&[f64], &[f64], &[f64]), usize, usize),
    ) -> (f64, f64) {
        let kfast = calc_kfast::<N>(
            &mut self.min_state,
            &mut self.max_state,
            inputs,
            i,
            k_period,
        );

        let old_k = self.prev_k.push_with_info(kfast);
        self.k_sum += kfast - old_k;
        let k = self.k_sum * self.k_multiplier;
        let old_d = self.prev_d.push_with_info(k);
        self.d_sum += k - old_d;

        (k, self.d_sum * self.d_multiplier)
    }
}

#[inline(always)]
fn calc_kfast<const N: usize>(
    min_state: &mut MinState<Warm>,
    max_state: &mut MaxState<Warm>,
    (high, low, close): (&[f64], &[f64], &[f64]),
    i: usize,
    period: usize,
) -> f64 {
    let shift = low.len() - close.len();

    let (min, _) =
        unsafe { min_state.calc_chuncked_unchecked::<N>((low, i + shift, (period, period - 1))) };
    let (max, _) =
        unsafe { max_state.calc_chuncked_unchecked::<N>((high, i + shift, (period, period - 1))) };

    100.0 * (close[i] - min) / (max - min).max(f64::EPSILON)
}
/// Performs the main calculation loop for the Stochastic Oscillator indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of three slices: `(high, low, close)`.
/// * `k_period` - The lookback period for the fast %K calculation.
/// * `start` - The starting index within `close` to begin output from.
/// * `multipliers` - A tuple `(k_multiplier, d_multiplier)` for the slow %K and %D averages.
/// * `state` - A mutable reference to the current `State`.
/// * `outputs` - A mutable tuple `(k_line, d_line)` of output slices.
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    k_period: usize,
    start: usize,
    state: &mut State<Warm>,
    (k_line, d_line): (&mut [f64], &mut [f64]),
) {
    for (j, i) in (start..inputs.2.len()).enumerate() {
        unsafe {
            (*k_line.get_unchecked_mut(j), *d_line.get_unchecked_mut(j)) =
                state.calc((inputs, i, k_period));
        }
    }
}

#[inline(always)]
pub fn multiplier(k_slow: usize, d_period: usize) -> (f64, f64) {
    (1.0 / k_slow as f64, 1.0 / d_period as f64)
}

pub struct Stoch;

impl Indicator<INPUTS, OPTIONS> for Stoch {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "stoch",
        full_name: "Stochastic Oscillator",
        indicator_type: IndicatorType::Momentum,
        inputs: &["high", "low", "close"],
        options: &["k_period", "k_slow", "d_period"],
        outputs: &["stoch_k", "stoch_d"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "stoch",
            label: "STOCH",
            display_type: DisplayType::Indicator,
            outputs: &["stoch_k", "stoch_d"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        (options[0] + options[1] + options[2]) as usize + 1
    }

    fn slot_lengths(data_len: usize, options: &[f64; OPTIONS]) -> Vec<usize> {
        let d_capacity = data_len - Self::min_data(options) + 1;
        vec![d_capacity + options[2] as usize, d_capacity]
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let k_period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let [high, low, close] = inputs;

        let (mut k_line, mut d_line, mut state, outputs, start);
        {
            let caps = Self::slot_lengths(high.len(), options);
            k_line = crate::uninit_vec!(f64, caps[0]);
            d_line = crate::uninit_vec!(f64, caps[1]);

            let k_slow = options[1] as usize;
            let d_period = options[2] as usize;
            let k_count;
            (state, k_count, start) =
                State::init_state((high, low, close), k_period, k_slow, d_period, &mut k_line);
            outputs = (&mut k_line[k_count..], d_line.as_mut_slice());
        }
        cycle((high, low, close), k_period, start, &mut state, outputs);

        Ok((
            vec![k_line, d_line],
            IndicatorState::new(state, high, low, k_period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stoch_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Stoch {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stoch_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Stoch {
    /// `STOCH(k, k_slow, d)` = slow %K (a `k_slow`-bar mean of fast %K) and %D (a
    /// `d`-bar mean of slow %K), where fast %K is the hand's `calc_kfast`:
    ///
    /// ```text
    /// kfast = 100 · (close − min) / max(max − min, ε)     rolls = (k, k−1)
    /// ```
    ///
    /// Hand facts encoded here, all read off `State::init_state`/`calc_chuncked`:
    /// · the two rolls are the SAME nodes `willr`/`donchian`/`max`/`min` declare —
    ///   `(k_period, k_period − 1)` over high and low — so a dashboard asking for
    ///   `stoch` and `willr` walks each column's window ONCE.
    /// · the divisor guard is `(max − min).max(f64::EPSILON)`: a `Max` against a
    ///   positive constant, NOT `willr`'s `WhereLt`. It floors rather than
    ///   substituting a whole result, so a flat bar (`max == min`, legal data)
    ///   yields `100·(close−min)/ε` here — a huge number, not `0.0` — and that is
    ///   what the hand prints, so the engine must not "improve" it.
    /// · `100.0 * (close − min) / den` is left-associative: `Div(Mul(100, sub))`,
    ///   not `Mul(100, Div)` (different bits).
    /// · slow %K is a mean (`k_sum · 1/k_slow`, the hand's `multiplier`), so both
    ///   smoothers are `RollSum{mean: true}` — the class E12 opened.
    /// · **the %K fold starts two bars after fast %K first exists.** The hand
    ///   seeds its roll states with `init_state(low, k_period + 1)` and begins the
    ///   loop at `i = k_period + 1`, so the first two published kfast values are
    ///   discarded and never enter `k_sum`. Hence `skip: 2` — and it is not
    ///   cosmetic: the ring drops by absolute bar, so the window at each printed
    ///   bar is the same set either way, but a fold that starts early reaches it
    ///   through extra add/subtract rounds, which is not the same float
    ///   (measured: two bars early flips 60% of the sums' last bits).
    /// · leads, verified against `::indicator()` row counts on four option sets:
    ///   fast %K exists at bar `k−1` (the roll), slow %K prints at `k + k_slow`,
    ///   %D at `k + k_slow + d`. The `skip` already moved the %K node's lead to
    ///   `k + k_slow`, so its sink needs no override; %D's natural lead is one bar
    ///   early (the hand buffers `d` %K values before its first write), so that
    ///   sink carries the explicit lead. `min_data = k + k_slow + d + 1` matches.
    ///
    /// `get_states` is DEFERRED (the plan runs and resumes bit-exact without it):
    /// the hand `State` holds two `Buffer`s — `prev_k` and `prev_d` — and those are
    /// exactly the rings `RollSum` already keeps in the state image, so the export
    /// is the `ChainState::Hma{ring, cursor}` precedent applied to `RollSum`, not a
    /// new mechanism. `k_sum`/`d_sum` and the two roll states are already
    /// reportable chain values.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        use crate::cse::{Node, Recipe, Sink};
        let k_period = options[0] as u32;
        let k_slow = options[1] as u32;
        let d_period = options[2] as u32;
        let look_back = k_period.saturating_sub(1);

        let mut r = Recipe::new();
        let high = r.add(Node::Input { stream: 0 });
        let low = r.add(Node::Input { stream: 1 });
        let close = r.add(Node::Input { stream: 2 });
        let mx = r.add(Node::RollMax {
            src: high,
            period: k_period,
            look_back,
        });
        let mn = r.add(Node::RollMin {
            src: low,
            period: k_period,
            look_back,
        });
        let dif = r.add(Node::Sub { a: mx, b: mn });
        let eps = r.constant(f64::EPSILON);
        let den = r.add(Node::Max { a: dif, b: eps });
        let mc = r.add(Node::Sub { a: close, b: mn });
        let c100 = r.constant(100.0);
        let num = r.add(Node::Mul { a: c100, b: mc });
        let kfast = r.add(Node::Div { a: num, b: den });
        // Slow %K: the hand discards fast %K's first two published values
        // (its roll seeding), hence `skip: 2`.
        let k_line = r.add(Node::RollSum {
            src: kfast,
            period: k_slow,
            mean: true,
            skip: 2,
            // The hand's init loop runs to bar `k + k_slow + d − 1`, past the
            // first full window, and inside it the eviction subtracts as its own
            // statement — so the steady `sum += v - old` only begins at
            // `k + k_slow + d`, exactly where %D starts printing.
            steady_from: k_period + k_slow + d_period,
        });
        // %D: a mean of the slow %K line, folding from that line's first
        // published bar — the hand's `prev_d` receives exactly those, and its
        // init loop never evicts, so the steady form applies throughout.
        let d_line = r.add(Node::RollSum {
            src: k_line,
            period: d_period,
            mean: true,
            skip: 0,
            steady_from: 0,
        });

        r.kind = "stoch";
        r.options = options.to_vec();
        r.outputs = vec![
            Sink::iface(k_line),
            Sink::iface_lead(d_line, k_period + k_slow + d_period),
        ];
        r
    }
}
