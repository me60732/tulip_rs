use crate::common::{validate_inputs, validate_options};
//use crate::indicators::aroon::State;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::max::{Max, State as MaxState};
use crate::indicators::min::State as MinState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    state: State<Warm>,
    high: Vec<f64>,
    low: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, high: &[f64], low: &[f64], period: usize) -> Self {
        Self {
            state,
            high: high[high.len() - period..].to_vec(),
            low: low[low.len() - period..].to_vec(),
            period,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        // Merge stored tails with new inputs.
        let [high, low, close] = *inputs;
        self.high.extend_from_slice(high);
        self.low.extend_from_slice(low);

        let (mut willr_line, (mut min_line, mut max_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    min_line: len,
                    max_line: len
                ),
            )
        };
        cycle_willr(
            (&self.high, &self.low, close),
            self.period,
            &mut self.state,
            &mut willr_line,
            (&mut min_line, &mut max_line),
        );

        self.high.drain(..self.high.len() - self.period);
        self.low.drain(..self.low.len() - self.period);

        Ok(vec![willr_line, min_line, max_line])
    }
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
}

impl State {
    pub fn new(min_state: (f64, usize), max_state: (f64, usize)) -> Self {
        State {
            min_state: MinState::new(min_state.0, min_state.1),
            max_state: MaxState::new(max_state.0, max_state.1),
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        period: usize,
        min_max: (&mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (min_line, max_line) = min_max;
        let look_back = period - 1;
        let mut min_state = MinState::init_state(low, look_back);
        let mut max_state = MaxState::init_state(high, look_back);

        let min = min_state.calc((low, look_back, (period, look_back))).0;
        let max = max_state.calc((high, look_back, (period, look_back))).0;
        crate::init_store_optional_outputs!(look_back, high.len(),
            min_line => min,
            max_line => max
        );
        State {
            min_state,
            max_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], f64, usize, (usize, usize));
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close, i, periods): Self::Inputs<'a>) -> Self::Outputs {
        // Update the minimum and maximum for the rolling window.
        let (min, _) = self.min_state.calc((low, i, periods));
        let (max, _) = self.max_state.calc((high, i, periods));

        if (max - min).abs() < f64::EPSILON {
            return (0.0, min, max);
        }

        (100.0 * (max - close) / (max - min), min, max)
    }
    /// Calculates Williams %R for a single bar using unchecked min/max access.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the rolling `State` (min and max states).
    /// * `high` - The full high price input slice.
    /// * `low` - The full low price input slice.
    /// * `close` - Reference to the current bar's close price.
    /// * `i` - The current index into `high` and `low`.
    /// * `periods` - A tuple of `(period, period - 1)` used by the min/max states.
    ///
    /// # Returns
    ///
    /// The Williams %R value for this bar.
    ///
    /// # Safety
    ///
    /// `i` and the look-back window must be within bounds of `high` and `low`.
    #[inline(always)]
    unsafe fn calc_unchecked(
        &mut self,
        inputs: (&[f64], &[f64], f64, usize, (usize, usize)),
    ) -> (f64, f64, f64) {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
    /// Calculates Williams %R for a single bar using unchecked min/max access.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the rolling `State` (min and max states).
    /// * `high` - The full high price input slice.
    /// * `low` - The full low price input slice.
    /// * `close` - Reference to the current bar's close price.
    /// * `i` - The current index into `high` and `low`.
    /// * `periods` - A tuple of `(period, period - 1)` used by the min/max states.
    ///
    /// # Returns
    ///
    /// The Williams %R value for this bar.
    ///
    /// # Safety
    ///
    /// `i` and the look-back window must be within bounds of `high` and `low`.
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (high, low, close, i, periods): (&[f64], &[f64], f64, usize, (usize, usize)),
    ) -> (f64, f64, f64) {
        // Update the minimum and maximum for the rolling window.
        let (min, _) = self
            .min_state
            .calc_chuncked_unchecked::<N>((low, i, periods));
        let (max, _) = self
            .max_state
            .calc_chuncked_unchecked::<N>((high, i, periods));

        if (max - min).abs() < f64::EPSILON {
            return (0.0, min, max);
        }
        (100.0 * (max - close) / (max - min), min, max)
    }
}

/// Iterates over the high, low, and close slices and computes Williams %R values.
///
/// # Arguments
///
/// * `high` - The full high price input slice.
/// * `low` - The full low price input slice.
/// * `close` - The close price slice to iterate over (already offset by `period`).
/// * `period` - The lookback period.
/// * `state` - Mutable reference to the rolling `State` (min and max states).
/// * `willr_line` - Mutable output slice for Williams %R values.
fn cycle_willr(
    (high, low, close): (&[f64], &[f64], &[f64]),
    period: usize,
    state: &mut State<Warm>,
    willr_line: &mut [f64],
    (min_line, max_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_min, want_max) = crate::calc_want_flags!(min_line, max_line);

    let periods = (period, period - 1);
    let mut i = period;
    for (j, (&close, willr)) in close.iter().zip(willr_line.iter_mut()).enumerate() {
        let (min, max);
        unsafe {
            (*willr, min, max) = state.calc_unchecked((high, low, close, i, periods));
        }

        if has_optional {
            crate::store_optional_outputs!(j,
                want_min, min_line => min,
                want_max, max_line => max
            );
        }

        i += 1;
    }
}

pub struct Willr;

impl Indicator<INPUTS, OPTIONS> for Willr {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "willr",
        full_name: "Williams %R",
        indicator_type: IndicatorType::Momentum,
        // Three inputs: high, low, close.
        inputs: &["high", "low", "close"],
        // One option: period.
        options: &["period"],
        outputs: &["willr"],
        optional_outputs: &["min", "max"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "willr",
                label: "WILLR",
                display_type: DisplayType::Indicator,
                outputs: &["willr"],
            },
            DisplayGroup {
                offset: None,
                id: "min_max",
                label: "Min & Max",
                display_type: DisplayType::Overlay,
                outputs: &["min", "max"],
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
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];

        let (mut willr_line, (mut min_line, mut max_line)) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            let min_max_capacity = Max::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    min_line: min_max_capacity,
                    max_line: min_max_capacity
                ),
            )
        };

        let mut state = State::init_state(high, low, period, (&mut min_line, &mut max_line));
        let optional_outputs = {
            let (min_offset, max_offset) =
                crate::slice_outputs_start!(willr_line.len(), min_line, max_line);
            (&mut min_line[min_offset..], &mut max_line[max_offset..])
        };

        cycle_willr(
            (high, low, &close[period..]),
            period,
            &mut state,
            &mut willr_line,
            optional_outputs,
        );

        Ok((
            vec![willr_line, min_line, max_line],
            IndicatorState::new(state, high, low, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::willr_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Willr {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::willr_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Willr {
    /// `WILLR` = `100·(max − close) / (max − min)`, zeroed when the range is
    /// degenerate (E9 M2). Hand facts encoded here:
    /// * window `(period, period − 1)` — max of HIGH, min of LOW, so this recipe
    ///   interns onto `donchianchannel`'s / `stoch`'s identical rolls at the same
    ///   period, but NOT onto `aroon`'s (its window is `(period, period)`).
    /// * the primary prints from bar `period` (`cycle_willr` starts there, and
    ///   `min_data = period + 1` sizes the row) — the roll nodes are defined one
    ///   bar earlier, hence the explicit sink lead;
    /// * the `min`/`max` optionals print ONE BAR EARLIER (hand `init_state`
    ///   already stores them at `look_back = period − 1`, then `slice_outputs_start!`
    ///   offsets the rest), so those sinks take the nodes' own lead;
    /// * `100.0 * (max − close) / (max − min)` is `Div(Mul(100, Sub), Sub)` —
    ///   Rust's left-to-right association, NOT `Mul(100, Div)` (different bits);
    /// * the `(max−min).abs() < f64::EPSILON → 0.0` guard is exactly
    ///   `WhereGe{a: |max−min|, b: EPSILON, t: div}` — `>=` takes the division,
    ///   `<` takes the pinned `+0.0` else arm. The division is computed
    ///   unconditionally (a `select`, both arms live) which is what `di`/`dx`
    ///   already do: a float divide by zero yields ±inf/NaN and is discarded.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        use crate::cse::{Node, Recipe, Sink};
        let period = options[0] as u32;
        let look_back = period - 1;
        let mut r = Recipe::new();
        let high = r.add(Node::Input { stream: 0 });
        let low = r.add(Node::Input { stream: 1 });
        let close = r.add(Node::Input { stream: 2 });
        let mx = r.add(Node::RollMax {
            src: high,
            period,
            look_back,
        });
        let mn = r.add(Node::RollMin {
            src: low,
            period,
            look_back,
        });
        let dif = r.add(Node::Sub { a: mx, b: mn });
        let adif = r.add(Node::Abs { src: dif });
        let mc = r.add(Node::Sub { a: mx, b: close });
        let c100 = r.constant(100.0);
        let num = r.add(Node::Mul { a: c100, b: mc });
        let q = r.add(Node::Div { a: num, b: dif });
        let eps = r.constant(f64::EPSILON);
        let wr = r.add(Node::WhereGe {
            a: adif,
            b: eps,
            t: q,
        });
        r.kind = "willr";
        r.options = options.to_vec();
        r.tail_inputs = vec![mn, mx];
        r.restorer = Some(restore_state);
        r.outputs = vec![Sink::iface_lead(wr, period)];
        r.optional_outputs = vec![Sink::iface(mn), Sink::iface(mx)];
        r
    }
}

/// Rebuild the hand `willr::State<Warm>` from the two roll chains (the
/// `high`/`low` tails are the engine's host anchor window).
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    let [crate::cse::ChainState::RollMin {
        value: min,
        trail: mt,
        ..
    }, crate::cse::ChainState::RollMax {
        value: max,
        trail: xt,
        ..
    }] = v.as_slice()
    else {
        return crate::cse::IndicatorStateBox::Other;
    };
    crate::cse::IndicatorStateBox::Willr(State {
        min_state: MinState::new(*min, *mt as usize).into_warm(),
        max_state: MaxState::new(*max, *xt as usize).into_warm(),
    })
}
