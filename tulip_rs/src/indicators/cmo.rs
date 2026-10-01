use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
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
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut cmo_line = crate::uninit_vec!(f64, inputs[0].len());

        self.real.extend_from_slice(inputs[0]);

        //let mut cmo_line: Vec<f64> = vec![0.0; capacity];

        cycle_cmo(&self.real, &mut self.state, self.period, &mut cmo_line);

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![cmo_line])
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct State<S = Cold> {
    pub up_sum: f64,
    pub down_sum: f64,
    pub prev: f64,
    pub drop_real: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(up_sum: f64, down_sum: f64, prev: f64, drop_real: f64) -> Self {
        Self {
            up_sum,
            down_sum,
            prev,
            drop_real,
            state: std::marker::PhantomData,
        }
    }
    /// Calculates the initial up and down sums for the CMO calculation.
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let [up, down] = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        State {
            up_sum,
            down_sum,
            prev: real[period],
            drop_real: real[0],
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (cur_real, old_real): Self::Inputs<'a>) -> f64 {
        let [old_up, old_down] = up_down(old_real, self.drop_real);
        self.drop_real = old_real;
        let [up, down] = up_down(cur_real, self.prev);
        self.prev = cur_real;
        self.up_sum += up - old_up;
        self.down_sum += down - old_down;

        100.0 * (self.up_sum - self.down_sum) / (self.up_sum + self.down_sum)
    }
}

/// Performs the main calculation loop for the CMO indicator.
///
/// # Arguments
///
/// * `real` - A slice of real values.
/// * `state` - Mutable reference to the CMO state (running up and down sums).
/// * `period` - The period for the CMO calculation.
/// * `cmo_line` - Mutable slice to write the CMO output values into.
fn cycle_cmo(real: &[f64], state: &mut State<Warm>, period: usize, cmo_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        let inputs = unsafe { (*real.get_unchecked(i), *real.get_unchecked(j)) };
        let cmo = state.calc(inputs);

        unsafe { *cmo_line.get_unchecked_mut(j) = cmo };
    }
}

#[inline(always)]
pub fn up_down(value: f64, prev_value: f64) -> [f64; 2] {
    let diff = value - prev_value;
    [diff.max(0.0), (-diff).max(0.0)]
}

pub struct Cmo;

impl Indicator<INPUTS, OPTIONS> for Cmo {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "cmo",
        indicator_type: IndicatorType::Momentum,
        full_name: "Chande Momentum Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["cmo"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "cmo",
            label: "CMO",
            display_type: DisplayType::Indicator,
            outputs: &["cmo"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 2
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut cmo_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(real, period);
        cycle_cmo(&real[1..], &mut state, period, &mut cmo_line);

        Ok((vec![cmo_line], IndicatorState::new(real, state, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cmo_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Cmo {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::cmo_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Cmo {
    /// `CMO(p)` = `100·(Σup − Σdown) / (Σup + Σdown)` over the movement of the
    /// last `p` bars — the first consumer of `RollSum`, a rolling sum over a
    /// DERIVED source (E12): what it accumulates is `up`/`down`, not a price, so
    /// the leaving term cannot come from the host anchor and must come from a
    /// ring of that series' own recent values.
    ///
    /// Hand facts encoded here, read against `cycle_cmo(&real[1..], …, period, …)`:
    /// · `up_down(v, prev) = [diff.max(0), (-diff).max(0)]` with `diff = v − prev`
    ///   and `prev = Lag{real, 1}` — the SAME nodes `rsi` builds (the negative arm
    ///   written `Max(Lag − real, 0)`, exact by IEEE's antisymmetric subtraction),
    ///   so an `rsi`+`cmo` query computes the movement ONCE; only the aggregation
    ///   differs: RSI Wilder-smooths these, CMO rolls them up.
    /// · `init_state` folds bars `1..=p` (its `skip(1)`), so a full window exists
    ///   at bar `p`. `RollSum` folds from bar 0 instead, and bar 0's movement is
    ///   `[0.0, 0.0]` through the `Lag` clamp — adding a zero to a sum changes
    ///   neither its value nor its rounding, so the two agree bit-for-bit.
    /// · for `i > p` the hand subtracts `up_down(real[i−p], real[i−p−1])`, which
    ///   IS the source's value exactly `p` bars back — the ring read at lag
    ///   `period`, applied as the hand writes it: `sum += (v − old)`, the
    ///   difference rounded first, never an `fma`.
    /// · `min_data = p + 2` while the first PRINTED bar is `p + 1` (the loop runs
    ///   `period..len` over `real[1..]`), hence the explicit sink lead.
    /// · the state also carries `prev` (last close) and `drop_real` (the close
    ///   `p` bars back). Both are captured as `Ema{period: 1}` — α = 1 makes that
    ///   recurrence the identity, so its chain slot holds exactly the input's
    ///   last value — the second over `Lag{real, period}`. `get_states` therefore
    ///   needs no new chain kind.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        let period = options[0] as u32;
        let mut r = crate::cse::Recipe::new();
        let real = r.add(crate::cse::Node::Input { stream: 0 });
        let l1 = r.add(crate::cse::Node::Lag { src: real, n: 1 });
        let diff = r.add(crate::cse::Node::Sub { a: real, b: l1 });
        let zero = r.constant(0.0);
        let up = r.add(crate::cse::Node::Max { a: diff, b: zero });
        // `-diff` in ONE op (IEEE subtraction is antisymmetric, both operands
        // finite) — and the same node `rsi` builds, so the pair shares it.
        let neg = r.add(crate::cse::Node::Sub { a: l1, b: real });
        let down = r.add(crate::cse::Node::Max { a: neg, b: zero });
        let up_sum = r.add(crate::cse::Node::RollSum {
            src: up,
            period,
            mean: false,
        });
        let down_sum = r.add(crate::cse::Node::RollSum {
            src: down,
            period,
            mean: false,
        });
        // `100.0 * (up_sum - down_sum) / (up_sum + down_sum)` parses left to
        // right, so the DIVIDE's numerator is the already-scaled difference.
        let hundred = r.constant(100.0);
        let num = r.add(crate::cse::Node::Sub {
            a: up_sum,
            b: down_sum,
        });
        let scaled = r.add(crate::cse::Node::Mul { a: hundred, b: num });
        let den = r.add(crate::cse::Node::Add {
            a: up_sum,
            b: down_sum,
        });
        let line = r.add(crate::cse::Node::Div { a: scaled, b: den });
        let lp = r.add(crate::cse::Node::Lag {
            src: real,
            n: period,
        });
        let prev_close = r.add(crate::cse::Node::Ema {
            src: real,
            period: 1,
        });
        let drop_close = r.add(crate::cse::Node::Ema { src: lp, period: 1 });
        r.kind = "cmo";
        r.options = options.to_vec();
        // hand `State` field order: up_sum, down_sum, prev, drop_real
        r.tail_inputs = vec![up_sum, down_sum, prev_close, drop_close];
        r.restorer = Some(restore_state);
        r.outputs
            .push(crate::cse::Sink::iface_lead(line, period + 1));
        r
    }
}

/// Rebuild the hand `cmo::State<Warm>`: the two raw sums plus the two lagged
/// closes its recurrence will need on the next bar.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    let [crate::cse::ChainState::Sma { sum: up_sum, .. }, crate::cse::ChainState::Sma { sum: down_sum, .. }, crate::cse::ChainState::Ema { value: prev, .. }, crate::cse::ChainState::Ema {
        value: drop_real, ..
    }] = v.as_slice()
    else {
        return crate::cse::IndicatorStateBox::Other;
    };
    crate::cse::IndicatorStateBox::Cmo(State::<Warm> {
        up_sum: *up_sum,
        down_sum: *down_sum,
        prev: *prev,
        drop_real: *drop_real,
        state: std::marker::PhantomData,
    })
}
