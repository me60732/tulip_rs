use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

pub(crate) use crate::indicators::cmo::up_down;
use crate::indicators::simd_indicators::wilders_simd::{
    multiplier_simd, SimdState as WildersSimdState,
};
pub use crate::indicators::wilders::multiplier;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::simd::Simd;
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut rsi_line = crate::uninit_vec!(f64, inputs[0].len());
        cycle_rsi(inputs[0], &mut rsi_line, self);

        Ok(vec![rsi_line])
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct State<S = Cold> {
    pub wilders_state: WildersSimdState<2>,
    pub prev_real: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(prev_real: f64, up_sum: f64, down_sum: f64, period: usize) -> Self {
        let multipliers = multiplier_simd([period, period]);
        Self {
            prev_real,
            wilders_state: WildersSimdState::new(Simd::from_array([up_sum, down_sum]), multipliers),
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut up_sum, mut down_sum) = (0.0, 0.0);
        //for i in 1..period+1 {
        for (i, &value) in real.iter().take(period + 1).enumerate().skip(1) {
            let prev_value = unsafe { *real.get_unchecked(i - 1) };
            let [up, down] = up_down(value, prev_value);
            up_sum += up;
            down_sum += down;
        }
        up_sum /= period as f64;
        down_sum /= period as f64;

        State {
            prev_real: real[period],
            wilders_state: WildersSimdState::new(
                Simd::from_array([up_sum, down_sum]),
                multiplier_simd([period, period]),
            ),
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, cur_real: Self::Inputs<'a>) -> Self::Outputs {
        let up_down = up_down(cur_real, self.prev_real);
        let [up_sum, down_sum] = self
            .wilders_state
            .calc(Simd::from_array(up_down))
            .to_array();

        self.prev_real = cur_real;

        100.0 * (up_sum / (up_sum + down_sum))
    }
}

/// Performs the main calculation loop for the RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `multiplier` - The smoothing multiplier for the RSI calculation.
/// * `rsi_line` - A mutable slice for storing the RSI output values.
/// * `state` - A mutable reference to the current RSI `State`.
fn cycle_rsi(real: &[f64], rsi_line: &mut [f64], state: &mut State<Warm>) {
    for i in 0..real.len() {
        unsafe { *rsi_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i)) };
    }
}

pub struct Rsi;
impl Indicator<INPUTS, OPTIONS> for Rsi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "rsi",
        indicator_type: IndicatorType::Momentum,
        full_name: "Relative Strength Index",
        inputs: &["real"],
        options: &["period"],
        outputs: &["rsi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "rsi",
            label: "RSI",
            display_type: DisplayType::Indicator,
            outputs: &["rsi"],
        }],
    };

    fn output_length(data_len: usize, options: &[f64; OPTIONS]) -> usize {
        data_len - Self::min_data(options)
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let mut rsi_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(inputs[0], period);

        cycle_rsi(&inputs[0][period + 1..], &mut rsi_line, &mut state);

        Ok((vec![rsi_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::rsi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Rsi {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::rsi_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Rsi {
    /// `RSI(p)` = `100 · up_avg / (up_avg + down_avg)`, each average being
    /// Wilder's MEAN over the per-bar movement. The hand keeps exactly ONE
    /// `WildersSimdState<2>` whose two lanes are these two chains, so the
    /// recipe's two `Wilders` nodes ARE the state.
    ///
    /// Hand facts encoded here:
    /// · `up_down(value, prev) = [diff.max(0), (-diff).max(0)]`, `diff =
    ///   value − prev`. The second arm is written `Max(Sub(prev, real), 0)`,
    ///   NOT `Max(Mul(-1, diff), 0)`: IEEE subtraction is exactly antisymmetric
    ///   (`round(a−b) = −round(b−a)`, both operands finite), so `prev − real`
    ///   IS `-diff` to the bit, one op cheaper, and it keeps the `Max` guard as
    ///   the second operand so a `+0.0` movement reproduces the hand's pattern.
    /// · `prev` is `Lag{real, 1}`, so movement is defined from bar 1, and
    ///   `init_state` folds `i in 1..=period` (its own `skip(1)`) then divides
    ///   by `period`. That is MEAN-mode `Wilders` with `feed: true` — fold the
    ///   source's PUBLISHED bars. Folding from bar 0 (the `atr` convention)
    ///   would add an `up[0]` the hand never sums.
    /// · the steady step `w.mul_add((p−1)/p, v·(1/p))` is bit-for-bit the
    ///   engine's mean `Wilders` (product single-rounded, then the fused add).
    /// · `min_data = period + 1`: the first printed bar is one after the seed
    ///   seam, hence the explicit sink lead.
    /// · the state's `prev_real` (last processed close) is carried as
    ///   `Ema{period: 1}`: α=1 makes that recurrence the identity, so its chain
    ///   slot IS the last close — `chain_of` reports chain kinds only, and this
    ///   keeps `get_states` uniform instead of adding an engine concept.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        let period = options[0] as u32;
        let mut r = crate::cse::Recipe::new();
        let real = r.add(crate::cse::Node::Input { stream: 0 });
        let prev = r.add(crate::cse::Node::Lag { src: real, n: 1 });
        let diff = r.add(crate::cse::Node::Sub { a: real, b: prev });
        let zero = r.constant(0.0);
        let up = r.add(crate::cse::Node::Max { a: diff, b: zero });
        // `-diff`, exactly and in one op (IEEE subtraction is antisymmetric).
        let neg = r.add(crate::cse::Node::Sub { a: prev, b: real });
        let down = r.add(crate::cse::Node::Max { a: neg, b: zero });
        let up_avg = r.add(crate::cse::Node::Wilders {
            src: up,
            period,
            composite: false,
            feed: true,
        });
        let down_avg = r.add(crate::cse::Node::Wilders {
            src: down,
            period,
            composite: false,
            feed: true,
        });
        let total = r.add(crate::cse::Node::Add {
            a: up_avg,
            b: down_avg,
        });
        let ratio = r.add(crate::cse::Node::Div {
            a: up_avg,
            b: total,
        });
        let hundred = r.constant(100.0);
        let line = r.add(crate::cse::Node::Mul {
            a: hundred,
            b: ratio,
        });
        let last_close = r.add(crate::cse::Node::Ema {
            src: real,
            period: 1,
        });
        r.kind = "rsi";
        r.options = options.to_vec();
        // hand `State` field order: the up lane, the down lane, `prev_real`
        r.tail_inputs = vec![up_avg, down_avg, last_close];
        r.restorer = Some(restore_state);
        r.outputs
            .push(crate::cse::Sink::iface_lead(line, period + 1));
        r
    }
}

/// Rebuild the hand `rsi::State<Warm>`: the two mean chains as one 2-lane
/// Wilder state plus `prev_real`. `multiplier_simd` is recomputed from the
/// period, never stored.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    let [crate::cse::ChainState::Wilders {
        value: up_sum,
        period,
    }, crate::cse::ChainState::Wilders {
        value: down_sum, ..
    }, crate::cse::ChainState::Ema {
        value: prev_real, ..
    }] = v.as_slice()
    else {
        return crate::cse::IndicatorStateBox::Other;
    };
    // The hand's `State::new` returns `State<Cold>`; `get_states` hands back the
    // warm form, so build it with the same two derived multipliers its
    // `init_state` uses (derivable, therefore never stored).
    let period = *period as usize;
    crate::cse::IndicatorStateBox::Rsi(State::<Warm> {
        wilders_state: WildersSimdState::new(
            Simd::from_array([*up_sum, *down_sum]),
            multiplier_simd([period, period]),
        ),
        prev_real: *prev_real,
        state: std::marker::PhantomData,
    })
}
