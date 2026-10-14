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
    /// · the state also carries `prev` (the last close, `State::init_state`'s
    ///   `real[period]`) and `drop_real` (the close `p` bars back, `real[0]`).
    ///   Both are `Mirror`s: `prev` mirrors `real`, `drop_real` mirrors the
    ///   `Lag{real, period}` node — a copy that keeps its last value in a state
    ///   word, so reporting them adds no arithmetic and no load the loop did not
    ///   already do. (`Ema{period: 1}` used to stand in for both; α=1 does return
    ///   its input, but only after running the EMA's multiply and fused add, and
    ///   `get_states` reported it as an EMA that CMO has never seen.)
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::cmo(options)
    }
}

/// Rebuild the hand `cmo::State<Warm>`: the two raw sums plus the two lagged
/// closes its recurrence will need on the next bar.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    use crate::cse::ChainState::{Mirror, Sma};
    let [Sma { sum: up_sum, .. }, Sma { sum: down_sum, .. }, Mirror { value: prev }, Mirror { value: drop_real }] =
        v.as_slice()
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

#[cfg(test)]
mod up_down_rewrite_proofs {
    //! `rsi`/`cmo` build their movement pair as `[max(d, 0), max(−d, 0)]`, `d` =
    //! value − prev — matching [`up_down`] arm for arm. An earlier form collapsed
    //! the second arm to `down = up − d` to halve the guard count (−12%/−16% when
    //! each guard was a mispredicting branch, §6.22). That is NOT the same
    //! function, and this module is the sweep that found it: over ~7,000 movement
    //! classes the collapsed form returns NaN where the hand writes `+0.0`, and a
    //! NaN entering a running sum never leaves.
    //!
    //! The engine's `Max` is also not Rust's `.max()`: `max_num` emits
    //! `select(d > 0, d, 0)`, which takes the SECOND operand on a tie or an
    //! unordered compare, while `f64::max` keeps `self`. That gap is where the two
    //! forms can still disagree — on the sign of a zero only — so `benign` states
    //! exactly what is tolerable and the sums test shows it is erased.

    /// The engine's lowering of `Node::Max { a, b }` (`codegen::max_num`'s
    /// `select(a > b, a, b)`), written independently of it on purpose.
    fn engine_max(a: f64, b: f64) -> f64 {
        if a > b {
            a
        } else {
            b
        }
    }

    /// Every class the rewrite could care about: both zeros, subnormals at both
    /// ends, the whole exponent span, the overflow boundary, infinities, both NaN
    /// shapes, and a deterministic pseudo-random sample of realistic movements.
    fn domain() -> Vec<f64> {
        let mut v: Vec<f64> = vec![
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1), // 5e-324, the smallest subnormal
            -f64::from_bits(1),
            f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1), // largest subnormal
            -f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1),
            f64::EPSILON,
            -f64::EPSILON,
            1.0,
            -1.0,
            f64::MAX,
            -f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            f64::from_bits(0x7ff8_0000_0000_0001),
            f64::from_bits(0xfff8_0000_0000_0000), // negative quiet NaN
        ];
        for k in -1074..1024 {
            let p = 2.0f64.powi(k);
            v.push(p);
            v.push(-p);
            v.push(p + p);
            v.push(-(p + p));
        }
        // Realistic movements (1e-6 .. 1e6), both signs, plus movements produced
        // BY a subtraction (the shape the real path takes).
        let mut state = 0x243f_6a88_85a3_08d3u64; // pi's digits: fixed => reproducible
        for i in 0..60_000u64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let frac = (state >> 11) as f64 / (1u64 << 53) as f64;
            let magnitude = 10f64.powf(frac * 12.0 - 6.0);
            let signed = if state & 1 == 0 {
                magnitude
            } else {
                -magnitude
            };
            v.push(signed);
            let other = (i as f64) * 0.01;
            v.push(signed - other);
            v.push(other - signed);
        }
        v
    }

    /// The accumulator side of the erasure test: every CLASS, not every value.
    /// Crossing 60k movements with 60k states is 100M adds in a debug profile
    /// (minutes), and the property under test — `x + (±0.0)` is `x`, a NaN stays
    /// a NaN — depends only on the class of `x`.
    fn sum_domain() -> Vec<f64> {
        let mut v = vec![
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            -f64::from_bits(1),
            f64::EPSILON,
            -f64::EPSILON,
            1.0,
            -1.0,
            f64::MAX,
            -f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
        ];
        for k in -1074..1024 {
            let p = 2.0f64.powi(k);
            v.push(p);
            v.push(-p);
        }
        v
    }

    /// What the recipes actually emit today: the hand's two arms, each a `Max`
    /// against `+0.0`, the second over `Sub(prev, real)` (`0.0 − d` here, with
    /// `prev` = 0). Kept separate from `up_down` on purpose: if a future edit
    /// collapses one arm into the other, this stops agreeing and the tests fail.
    fn recipe_pair(value: f64, prev: f64) -> [f64; 2] {
        let diff = value - prev;
        let up = engine_max(diff, 0.0);
        let down = engine_max(prev - value, 0.0);
        [up, down]
    }

    /// A difference is acceptable ONLY if it is the sign of a zero (`-0.0` vs
    /// `+0.0`), which no consumer of these two values can observe as anything but
    /// the same number, or both sides NaN. Anything else — a zero where the hand
    /// has an infinity, a NaN where it has a number — is a real divergence.
    fn benign(a: f64, b: f64) -> bool {
        (a == 0.0 && b == 0.0) || (a.is_nan() && b.is_nan())
    }

    #[test]
    fn recipe_pair_matches_the_hand_on_every_class() {
        let dom = domain();
        let mut diffs = Vec::new();
        for &d in &dom {
            // `up_down(d, 0.0)` reproduces the hand for movement `d`, because
            // `d − 0.0` is `d` for every class including both zeros and ∞.
            let hand = super::up_down(d, 0.0);
            let recipe = recipe_pair(d, 0.0);
            for k in 0..2 {
                if hand[k].to_bits() != recipe[k].to_bits() && !benign(hand[k], recipe[k]) {
                    diffs.push((d, k, hand[k], recipe[k]));
                }
            }
        }
        assert!(
            diffs.is_empty(),
            "{} divergences beyond signed zeros (first 5: {:?})",
            diffs.len(),
            &diffs[..diffs.len().min(5)]
        );
    }

    #[test]
    fn the_superseded_one_guard_form_is_what_this_sweep_caught() {
        // Negative control, so the suite records WHY the second arm exists:
        // `down = up − diff` agrees with the hand on every finite movement and
        // then returns NaN at `+∞`, where the hand writes `+0.0`. If a future
        // "optimization" re-collapses the arms, the first test fails; this one
        // says what it would break.
        let caught = |d: f64| {
            let up = engine_max(d, 0.0);
            [up, up - d]
        };
        for &d in &[f64::INFINITY, f64::NAN, -f64::NAN] {
            let hand = super::up_down(d, 0.0);
            let collapsed = caught(d);
            assert!(
                !benign(hand[1], collapsed[1]),
                "expected the one-guard form to diverge at d={d:?}, got hand={:?} \
                 collapsed={:?} — the sweep's finding no longer holds",
                hand[1],
                collapsed[1]
            );
        }
        // …and agrees everywhere the movements are finite, which is exactly why
        // the price-regime parity matrices never saw it.
        for &d in &[0.0, -0.0, 1.0, -1.0, f64::MIN_POSITIVE, -1.0e308, 1.0e308] {
            let hand = super::up_down(d, 0.0);
            let collapsed = caught(d);
            assert!(
                benign(hand[1], collapsed[1]) || hand[1].to_bits() == collapsed[1].to_bits(),
                "finite case d={d:?} unexpectedly diverges: hand={:?} collapsed={:?}",
                hand[1],
                collapsed[1]
            );
        }
    }

    #[test]
    fn any_signed_zero_difference_is_erased_by_the_sum_it_feeds() {
        // Neither `up` nor `down` is ever published: both are only CONSUMED by a
        // running sum (`Wilders`, `RollSum`). So where the pair may differ in the
        // SIGN of a zero, the sums must still agree bit for bit — or differ only
        // in a way that cannot propagate. Movements run over the full domain,
        // accumulator states over every CLASS (`sum_domain`); see its comment for
        // why the cross product is sized that way.
        let dom = domain();
        let sums = sum_domain();
        let mut bad = Vec::new();
        for &d in &dom {
            let hand = super::up_down(d, 0.0);
            let recipe = recipe_pair(d, 0.0);
            for &s in &sums {
                for k in 0..2 {
                    let a = s + hand[k];
                    let b = s + recipe[k];
                    if a.to_bits() != b.to_bits() && !(benign(a, b)) {
                        bad.push((s, d, k, a, b));
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "{} sums differ beyond ±0 (first 5: {:?})",
            bad.len(),
            &bad[..bad.len().min(5)]
        );
    }

    #[test]
    fn the_sweep_covers_the_classes_it_claims() {
        // Control: a domain that silently degenerated would prove nothing.
        let d = domain();
        assert!(d.iter().any(|&x| x == 0.0 && x.is_sign_negative()));
        assert!(d.iter().copied().any(f64::is_nan));
        assert!(d.iter().copied().any(|x| x.is_infinite()));
        assert!(d.iter().copied().any(f64::is_subnormal));
        assert!(d.len() > 6_000, "domain unexpectedly small: {}", d.len());
    }
}
