use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::{
    max::State as MaxState,
    min::{Min, State as MinState},
};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    high: Vec<f64>,
    low: Vec<f64>,
    state: State<Warm>,
}
impl IndicatorState {
    pub fn new(
        high: &[f64],
        low: &[f64],
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        state: State<Warm>,
    ) -> Self {
        Self {
            high: high[high.len() - periods.2 .1..].to_vec(),
            low: low[low.len() - periods.2 .1..].to_vec(),
            periods,
            state,
        }
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close] = *inputs;
        self.high.extend_from_slice(high);
        self.low.extend_from_slice(low);

        let (mut conversion_line, mut base_line, mut span_a_line, mut span_b_line, lagging_span) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                lagging_output(close, optional_outputs),
            )
        };
        cycle(
            (&self.high, &self.low),
            self.periods,
            &mut self.state,
            (
                &mut conversion_line,
                &mut base_line,
                &mut span_a_line,
                &mut span_b_line,
            ),
        );

        self.high.drain(..self.high.len() - self.periods.2 .1);
        self.low.drain(..self.low.len() - self.periods.2 .1);

        Ok(vec![
            conversion_line,
            base_line,
            span_a_line,
            span_b_line,
            lagging_span,
        ])
    }
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_min_state: MinState<S>,
    pub short_max_state: MaxState<S>,
    pub medium_min_state: MinState<S>,
    pub medium_max_state: MaxState<S>,
    pub long_min_state: MinState<S>,
    pub long_max_state: MaxState<S>,
}

impl State {
    pub fn new(
        high: &[f64],
        low: &[f64],
        periods: ((usize, usize), (usize, usize), (usize, usize)),
    ) -> Self {
        Self {
            short_min_state: MinState::new(low[0], periods.0 .1),
            short_max_state: MaxState::new(high[0], periods.0 .1),
            medium_min_state: MinState::new(low[0], periods.1 .1),
            medium_max_state: MaxState::new(high[0], periods.1 .1),
            long_min_state: MinState::new(low[0], periods.2 .1),
            long_max_state: MaxState::new(high[0], periods.2 .1),
        }
    }
    pub fn init_state(
        (high, low): (&[f64], &[f64]),
        (short_periods, long_periods, ultra_periods): (
            (usize, usize),
            (usize, usize),
            (usize, usize),
        ),
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let mut short_min_state = MinState::init_state(low, short_periods.1);
        let mut short_max_state = MaxState::init_state(high, short_periods.1);
        let mut medium_min_state = MinState::init_state(low, long_periods.1);
        let mut medium_max_state = MaxState::init_state(high, long_periods.1);
        let long_min_state = MinState::init_state(low, ultra_periods.1);
        let long_max_state = MaxState::init_state(high, ultra_periods.1);

        let (conversion_line, base_line, span_a_line) = out_vecs;

        let (mut base, mut span_a) = (0.0, 0.0);
        let len = high.len();
        for i in short_periods.1..ultra_periods.1 {
            let short_min = short_min_state.calc((low, i, short_periods)).0;
            let short_max = short_max_state.calc((high, i, short_periods)).0;
            let conversion = 0.5 * (short_min + short_max);

            if i >= long_periods.1 {
                let medium_min = medium_min_state.calc((low, i, long_periods)).0;
                let medium_max = medium_max_state.calc((high, i, long_periods)).0;
                base = 0.5 * (medium_min + medium_max);
                span_a = 0.5 * (conversion + base);
            }
            crate::init_store_optional_outputs!(i, len,
                conversion_line => conversion,
                base_line => base,
                span_a_line => span_a

            );
        }
        State {
            short_min_state,
            short_max_state,
            medium_min_state,
            medium_max_state,
            long_min_state,
            long_max_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (
        &'a [f64],
        &'a [f64],
        usize,
        ((usize, usize), (usize, usize), (usize, usize)),
    );
    type Outputs = (f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, i, periods): Self::Inputs<'a>) -> Self::Outputs {
        let long_min = self.long_min_state.calc((low, i, periods.2)).0;
        let medium_min = self.medium_min_state.calc((low, i, periods.1)).0;
        let short_min = self.short_min_state.calc((low, i, periods.0)).0;

        let long_max = self.long_max_state.calc((high, i, periods.2)).0;
        let medium_max = self.medium_max_state.calc((high, i, periods.1)).0;
        let short_max = self.short_max_state.calc((high, i, periods.0)).0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
    #[inline(always)]
    unsafe fn calc_unchecked(&mut self, inputs: Self::Inputs<'_>) -> Self::Outputs {
        self.calc_chuncked_unchecked::<1, 4, 4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const CS: usize, const CM: usize, const CL: usize>(
        &mut self,
        (high, low, i, periods): (
            &[f64],
            &[f64],
            usize,
            ((usize, usize), (usize, usize), (usize, usize)),
        ),
    ) -> (f64, f64, f64, f64) {
        let long_min = self
            .long_min_state
            .calc_chuncked_unchecked::<CL>((low, i, periods.2))
            .0;
        let long_max = self
            .long_max_state
            .calc_chuncked_unchecked::<CL>((high, i, periods.2))
            .0;

        let medium_min = self
            .medium_min_state
            .calc_chuncked_unchecked::<CM>((low, i, periods.1))
            .0;
        let medium_max = self
            .medium_max_state
            .calc_chuncked_unchecked::<CM>((high, i, periods.1))
            .0;

        let short_min = self
            .short_min_state
            .calc_chuncked_unchecked::<CS>((low, i, periods.0))
            .0;
        let short_max = self
            .short_max_state
            .calc_chuncked_unchecked::<CS>((high, i, periods.0))
            .0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
}
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
fn lagging_output(close: &[f64], optional_outputs: Option<&[bool]>) -> Vec<f64> {
    if let Some(oo) = optional_outputs {
        if oo.len() > 0 && oo[0] {
            close.to_vec()
        } else {
            Vec::<f64>::with_capacity(0)
        }
    } else {
        Vec::<f64>::with_capacity(0)
    }
}

fn cycle(
    (high, low): (&[f64], &[f64]),
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    state: &mut State<Warm>,
    (conversion_line, base_line, span_a_line, span_b_line): (
        &mut [f64],
        &mut [f64],
        &mut [f64],
        &mut [f64],
    ),
) {
    for (j, i) in (periods.2 .1..high.len()).enumerate() {
        unsafe {
            (
                *conversion_line.get_unchecked_mut(j),
                *base_line.get_unchecked_mut(j),
                *span_a_line.get_unchecked_mut(j),
                *span_b_line.get_unchecked_mut(j),
            ) = state.calc_unchecked((high, low, i, periods))
        }
    }
}

pub struct Ichimoku;

impl Indicator<INPUTS, OPTIONS> for Ichimoku {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ichimoku",
        full_name: "Ichimoku",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["short_period", "long_period"],
        outputs: &["conversion", "base", "leading_span_a", "leading_span_b"],
        optional_outputs: &["lagging_span"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "Conversion_Base",
                label: "Tenkan-sel & Kijun-sen",
                display_type: DisplayType::Overlay,
                outputs: &["conversion", "base"],
            },
            DisplayGroup {
                offset: Some("+long_period"),
                id: "leading",
                label: "Senkou Span A & Senkou Span B",
                display_type: DisplayType::Overlay,
                outputs: &["leading_span_a", "leading_span_b"],
            },
            DisplayGroup {
                offset: Some("-long_period"),
                id: "close",
                label: "Chikou Span",
                display_type: DisplayType::Price,
                outputs: &["lagging_span"],
            },
        ],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        Min::min_data(&[options[1] * 2.0]) + options[1] as usize
    }

    fn slot_lengths(data_len: usize, options: &[f64; OPTIONS]) -> Vec<usize> {
        let ultra_long = options[1] as usize * 2;

        vec![
            Min::output_length(data_len, &[options[0]]),
            Min::output_length(data_len, &[options[1]]),
            Min::output_length(data_len, &[ultra_long as f64]),
            Min::output_length(data_len, &[ultra_long as f64]),
            data_len,
        ]
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let [high, low, close] = *inputs;

        let periods = {
            let (short_period, long_period) = (options[0] as usize, options[1] as usize);
            let ultra_long = long_period * 2;
            (
                (short_period, short_period - 1),
                (long_period, long_period - 1),
                (ultra_long, ultra_long - 1),
            )
        };
        let (mut conversion_line, mut base_line, mut span_a_line, mut span_b_line, lagging_span) = {
            let caps = Self::slot_lengths(high.len(), options);
            (
                crate::uninit_vec!(f64, caps[0]),
                crate::uninit_vec!(f64, caps[1]),
                crate::uninit_vec!(f64, caps[2]),
                crate::uninit_vec!(f64, caps[3]),
                lagging_output(close, optional_outputs),
            )
        };
        let mut state = State::init_state(
            (high, low),
            periods,
            (&mut conversion_line, &mut base_line, &mut span_a_line),
        );
        let outputs = {
            let offset = crate::slice_outputs_start!(
                span_b_line.len(),
                conversion_line,
                base_line,
                span_a_line
            );
            (
                &mut conversion_line[offset.0..],
                &mut base_line[offset.1..],
                &mut span_a_line[offset.2..],
                span_b_line.as_mut_slice(),
            )
        };
        cycle((high, low), periods, &mut state, outputs);

        Ok((
            vec![
                conversion_line,
                base_line,
                span_a_line,
                span_b_line,
                lagging_span,
            ],
            IndicatorState::new(high, low, periods, state),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ichimoku_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Ichimoku {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ichimoku_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Ichimoku {
    /// `ICHIMOKU` — six rolls (short / medium / ultra × max / min) over high and
    /// low, plus four `0.5·(x + y)` combines (E9 M4). It is the family's densest
    /// nesting case: three strictly increasing windows per column, so each
    /// column chains twice and the rescans cost `s`, `l − s`, `2l − l` loads
    /// instead of `s + l + 2l` (for 9/26: 9 + 17 + 26 rather than 9 + 26 + 52).
    ///
    /// Hand facts encoded here:
    /// * every window is CANONICAL `(period, period − 1)` —
    ///   `(s, s−1) (l, l−1) (2l, 2l−1)` — so unlike `aroon` no early guard here;
    /// * publish leads come from `slot_lengths`, NOT from each node's
    ///   defined-from: conversion prints from bar `s−1`, base from `l−1`, and
    ///   BOTH spans from `2l−1`. `span_a = 0.5·(conversion + base)` is
    ///   computable from `l−1`; the hand withholds it to `2l−1` on purpose
    ///   (Senkou Span A's own warm-up convention), which is a sink-lead fact —
    ///   `iface_lead` clamps UP to defined-from, so a later lead is legal and
    ///   no padding feature is needed;
    /// * `lagging_span` is a verbatim clone of the caller's `close`, enters no
    ///   loop, and is therefore NOT declared here — the hand exposes it as an
    ///   optional so the CALLER can decline the copy, and the engine has no
    ///   business cloning a column the caller already owns. Consequence worth
    ///   knowing: the plan reads only 2 fields, i.e. ichimoku's math never
    ///   touches close, so `n_streams()` is `2 · nassets` (the hand's `INPUTS`
    ///   is 3 only because of that optional).
    /// * the `+long_period` / `−long_period` `DisplayGroup` offsets are
    ///   presentation metadata applied by the display layer; they are not data
    ///   shifts and the planner never sees them.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        use crate::cse::{Node, Recipe, Sink};
        let short = options[0] as u32;
        let medium = options[1] as u32;
        let long = medium * 2;
        let mut r = Recipe::new();
        let high = r.add(Node::Input { stream: 0 });
        let low = r.add(Node::Input { stream: 1 });

        // Narrow → wide on purpose: the planner links each roll to the widest
        // still-narrower roll on the same column and direction, so this order
        // also matches the arena's.
        let s_min = r.add(Node::RollMin {
            src: low,
            period: short,
            look_back: short - 1,
        });
        let s_max = r.add(Node::RollMax {
            src: high,
            period: short,
            look_back: short - 1,
        });
        let m_min = r.add(Node::RollMin {
            src: low,
            period: medium,
            look_back: medium - 1,
        });
        let m_max = r.add(Node::RollMax {
            src: high,
            period: medium,
            look_back: medium - 1,
        });
        let l_min = r.add(Node::RollMin {
            src: low,
            period: long,
            look_back: long - 1,
        });
        let l_max = r.add(Node::RollMax {
            src: high,
            period: long,
            look_back: long - 1,
        });

        let half = r.constant(0.5);
        let conv_sum = r.add(Node::Add { a: s_min, b: s_max });
        let conversion = r.add(Node::Mul {
            a: half,
            b: conv_sum,
        });
        let base_sum = r.add(Node::Add { a: m_min, b: m_max });
        let base = r.add(Node::Mul {
            a: half,
            b: base_sum,
        });
        let span_a_sum = r.add(Node::Add {
            a: conversion,
            b: base,
        });
        let span_a = r.add(Node::Mul {
            a: half,
            b: span_a_sum,
        });
        let span_b_sum = r.add(Node::Add { a: l_min, b: l_max });
        let span_b = r.add(Node::Mul {
            a: half,
            b: span_b_sum,
        });

        r.kind = "ichimoku";
        r.options = options.to_vec();
        // hand `State` field order
        r.tail_inputs = vec![s_min, s_max, m_min, m_max, l_min, l_max];
        r.restorer = Some(restore_state);
        r.outputs = vec![
            Sink::iface_lead(conversion, short - 1),
            Sink::iface_lead(base, medium - 1),
            Sink::iface_lead(span_a, long - 1),
            Sink::iface_lead(span_b, long - 1),
        ];
        r
    }
}

/// Rebuild the hand `ichimoku::State<Warm>` — six `{value, trail}` pairs, exactly
/// the hand's six `MinState`/`MaxState` fields. No derived constant to recompute
/// (there is no multiplier), and the `high`/`low` tails the hand
/// `IndicatorState` keeps are the engine's host anchor window.
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    use crate::cse::{ChainState, IndicatorStateBox};
    let [ChainState::RollMin {
        value: smin,
        trail: smin_t,
        ..
    }, ChainState::RollMax {
        value: smax,
        trail: smax_t,
        ..
    }, ChainState::RollMin {
        value: mmin,
        trail: mmin_t,
        ..
    }, ChainState::RollMax {
        value: mmax,
        trail: mmax_t,
        ..
    }, ChainState::RollMin {
        value: lmin,
        trail: lmin_t,
        ..
    }, ChainState::RollMax {
        value: lmax,
        trail: lmax_t,
        ..
    }] = v.as_slice()
    else {
        return IndicatorStateBox::Other;
    };
    IndicatorStateBox::Ichimoku(State {
        short_min_state: MinState::new(*smin, *smin_t as usize).into_warm(),
        short_max_state: MaxState::new(*smax, *smax_t as usize).into_warm(),
        medium_min_state: MinState::new(*mmin, *mmin_t as usize).into_warm(),
        medium_max_state: MaxState::new(*mmax, *mmax_t as usize).into_warm(),
        long_min_state: MinState::new(*lmin, *lmin_t as usize).into_warm(),
        long_max_state: MaxState::new(*lmax, *lmax_t as usize).into_warm(),
    })
}
