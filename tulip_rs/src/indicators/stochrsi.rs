use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::max::State as MaxState;
use crate::indicators::min::State as MinState;
use crate::indicators::rsi::{Rsi, State as RsiState};
use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    period: usize,
    state: State<Warm>,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, period: usize) -> Self {
        Self { period, state }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let capacity = inputs[0].len();
        let mut rsi_line = crate::init_optional_outputs!(
            optional_outputs, &[false],
            rsi_line: capacity
        );

        let real = inputs[0];
        let mut stochrsi_line = vec![0.0; capacity];

        cycle_stochrsi(
            real,
            self.period,
            &mut stochrsi_line,
            &mut self.state,
            &mut rsi_line,
        );

        Ok(vec![stochrsi_line, rsi_line])
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: MirrorBuffer<S>,
    pub rsi_state: RsiState<S>,
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
}
impl State {
    pub fn init_state(real: &[f64], period: usize, rsi_line: &mut [f64]) -> State<Warm> {
        let mut rsi_state = RsiState::init_state(real, period);
        let mut buffer = MirrorBuffer::new(period);
        let [up_sum, down_sum] = rsi_state.wilders_state.wilders.to_array();
        let mut rsi = 100.0 * (up_sum / (up_sum + down_sum));
        buffer.push(rsi);
        let mut min_state = MinState::new(rsi, period);
        let mut max_state = MaxState::new(rsi, period);
        //let multiplier = multiplier(period);
        let mut i = period + 1;
        while buffer.get_count() < buffer.get_capacity() {
            rsi = rsi_state.calc(real[i]);
            buffer.push(rsi);
            buffer.min(&mut min_state, rsi);
            buffer.max(&mut max_state, rsi);
            crate::init_store_optional_outputs!(i, real.len(), rsi_line => rsi);
            i += 1;
        }
        State {
            min_state: min_state.into_warm(),
            max_state: max_state.into_warm(),
            rsi_state,
            buffer: buffer.into_full(),
        }
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, usize);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc(&mut self, (real, period): Self::Inputs<'_>) -> Self::Outputs {
        let rsi = self.rsi_state.calc(real);
        self.buffer.push(rsi);

        let (min, _) = self.buffer.min(&mut self.min_state, rsi, period);
        let (max, _) = self.buffer.max(&mut self.max_state, rsi, period);

        let kdif = max - min;
        let kfast = if kdif < f64::EPSILON {
            0.0
        } else {
            100.0 * (rsi - min) / kdif
        };

        (kfast, rsi)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn calc_chunked<const N: usize>(&mut self, (real, period): (f64, usize)) -> (f64, f64) {
        let rsi = self.rsi_state.calc(real);
        self.buffer.push(rsi);

        let (min, _) = self
            .buffer
            .min_chuncked::<N>(&mut self.min_state, rsi, period);
        let (max, _) = self
            .buffer
            .max_chuncked::<N>(&mut self.max_state, rsi, period);

        let kdif = max - min;
        let kfast = if kdif < f64::EPSILON {
            0.0
        } else {
            100.0 * (rsi - min) / kdif
        };

        (kfast, rsi)
    }
}
/// Performs the main calculation loop for the Stochastic RSI indicator.
///
/// # Arguments
///
/// * `real` - A slice of real prices.
/// * `multiplier` - The EMA multiplier derived from the period.
/// * `period` - The period for the Stochastic RSI calculation.
/// * `stochrsi_line` - A mutable slice for storing the Stochastic RSI output values.
/// * `state` - A mutable reference to the current indicator state.
/// * `rsi_line` - A mutable slice for storing the optional RSI output values.
fn cycle_stochrsi(
    real: &[f64],
    period: usize,
    stochrsi_line: &mut [f64],
    state: &mut State<Warm>,
    rsi_line: &mut [f64],
) {
    let (_, want_rsi) = crate::calc_want_flags!(rsi_line);

    for i in 0..real.len() {
        let val = unsafe { *real.get_unchecked(i) };

        let (kfast, rsi) = state.calc_chunked::<8>((val, period));

        unsafe { *stochrsi_line.get_unchecked_mut(i) = kfast };
        crate::store_optional_outputs!(i,
            want_rsi, rsi_line => rsi
        );
    }
}

pub struct StochRsi;

impl Indicator<INPUTS, OPTIONS> for StochRsi {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "stochrsi",
        full_name: "Stochastic RSI",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["stochrsi"],
        optional_outputs: &["rsi"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "stochrsi",
            label: "STOCHRSI",
            display_type: DisplayType::Indicator,
            outputs: &["stochrsi", "rsi"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        (options[0]) as usize * 2 + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let capacity = Self::output_length(real.len(), options);
        let rsi_capacity = Rsi::output_length(real.len(), options);
        let mut stochrsi_line = crate::uninit_vec!(f64, capacity); //vec![0.0; capacity]; // Vec::with_capacity(capacity);
        let mut rsi_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            rsi_line: rsi_capacity
        );
        let mut state = State::init_state(real, period, &mut rsi_line);
        let rsi = {
            let offset = crate::slice_outputs_start!(stochrsi_line.len(), rsi_line);
            &mut rsi_line[offset..]
        };
        let real = &real[period * 2..];

        cycle_stochrsi(real, period, &mut stochrsi_line, &mut state, rsi);

        Ok((
            vec![stochrsi_line, rsi_line],
            IndicatorState::new(state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stochrsi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for StochRsi {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::stochrsi_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for StochRsi {
    /// `STOCHRSI(p)` = `100·(rsi − min) / (max − min)` over the last `p` RSI
    /// values, with the RSI line itself an optional output — ONE option driving
    /// both the Wilders smoothing and the stochastic window, exactly as the hand
    /// wires it (`RsiState::init_state(real, period)` and `MirrorBuffer::new(period)`
    /// take the same number).
    ///
    /// This is the first consumer of the DERIVED-SOURCE roll (E13/§6.29): the
    /// window rolls over a computed series, so there is no host anchor to read
    /// `i − look_back … i` from and the roll gets a ring of the source's own
    /// node-values, exactly like `RollSum`'s. Consequences kept visible here:
    /// its rescan walks that ring with masked addressing (scalar, because a
    /// 2-slot pair can straddle the wrap), and it takes part in no seed link —
    /// `stochrsi`'s own pair is a max and a min, whose scans anti-correlate.
    ///
    /// Hand facts encoded here:
    /// · the RSI half is `sub_at::<Rsi>`, so a query asking for `rsi` and
    ///   `stochrsi` shares the whole Wilder pair, the division and the ×100 —
    ///   stochrsi's optional RSI row and standalone rsi's primary row are then
    ///   the SAME stored stream;
    /// · `if kdif < ε { 0.0 } else { 100·(rsi−min)/kdif }` is `WhereLt` on the raw
    ///   difference (no `Abs`: `max − min ≥ 0` by construction) with the division
    ///   parsed left to right, `Div(Mul(100, sub), kdif)`;
    /// · leads: the rsi node exists from bar `p` (its seed mean, which IS the
    ///   buffer's first push), the roll's window is complete at `2p − 1`, and the
    ///   hand PRINTS from `2p` (`init_state` stops when the buffer fills at
    ///   `2p − 1`, `cycle_stochrsi` starts at `2p`; `min_data = 2p + 1`), hence the
    ///   explicit sink lead. The rsi row keeps rsi's own `p + 1` lead, matching
    ///   `Rsi::output_length`.
    ///
    /// `get_states` is DEFERRED like `stoch`'s/`vhf`'s: the hand state's buffer is
    /// this plan's ring and `rsi_state` is already reportable, so the export is the
    /// one `ChainState` ring mechanism those two also wait on.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        use crate::cse::{Node, Recipe, Sink};
        let period = options[0] as u32;
        let mut r = Recipe::new();
        let real = r.add(Node::Input { stream: 0 });
        let rsi = r.sub_at::<crate::indicators::rsi::Rsi>(&[period as f64], &[real]);
        let mx = r.add(Node::RollMax {
            src: rsi,
            period,
            look_back: period.saturating_sub(1),
        });
        let mn = r.add(Node::RollMin {
            src: rsi,
            period,
            look_back: period.saturating_sub(1),
        });
        let kdif = r.add(Node::Sub { a: mx, b: mn });
        let sub = r.add(Node::Sub { a: rsi, b: mn });
        let c100 = r.constant(100.0);
        let num = r.add(Node::Mul { a: c100, b: sub });
        let q = r.add(Node::Div { a: num, b: kdif });
        let eps = r.constant(f64::EPSILON);
        // `if kdif < ε { 0.0 } else { q }` is `WhereLt` directly — the `<`-form
        // mirror of `WhereGe`, which is what the hand actually writes. The reason
        // it must be `<` and not `>=`: on a FLAT series the RSI line is exactly
        // `0/0 = NaN`, so `kdif = max − min` is NaN, and an IEEE compare is false in
        // BOTH directions there. The hand's `<` test therefore falls to the ELSE arm
        // (the division) and keeps the NaN, which is what it prints; `WhereGe`'s
        // false branch is the pinned `+0.0` and swallows it (the parity test failed
        // on `stochrsi flat p3` until this node replaced a `0 − maxNum(0 − kdif, −∞)`
        // mapping that cost three extra ops to buy the same NaN propagation).
        let line = r.add(Node::WhereLt {
            a: kdif,
            b: eps,
            t: q,
        });

        r.kind = "stochrsi";
        r.options = options.to_vec();
        r.outputs.push(Sink::iface_lead(line, period * 2));
        r.optional_outputs = vec![Sink::iface_lead(rsi, period + 1)];
        r
    }
}
