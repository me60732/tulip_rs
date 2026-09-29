use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::di::State as DiState;
use crate::indicators::tr::Tr;
pub use crate::indicators::wilders::multiplier;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub DiState<S>);
impl<S> Deref for State<S> {
    type Target = DiState<S>;
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
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (_, _, atr, tr) = self.calc_diup_didown(inputs);

        let dx = self.calc_dx();

        (dx, atr, tr)
    }
}
impl State<Cold> {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        State(DiState::init_state(high, low, close, period, tr_line))
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn calc_dx(&mut self) -> f64 {
        let di_up = self.di_state.dmup; // / state.atr_state.atr;
        let di_down = self.di_state.dmdown; // / state.atr_state.atr;

        let dm_diff = (di_up - di_down).abs();
        let dm_sum = di_up + di_down;
        (dm_diff * 100.0 / dm_sum).max(0.0)
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        //let mut dx_line = vec![0.0; capacity];
        let (mut dx_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            dx_line = crate::uninit_vec!(f64, capacity);
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        let [high, low, close] = inputs;
        cycle(
            high,
            low,
            close,
            self,
            (&mut dx_line, &mut atr_line, &mut tr_line),
        );
        Ok(vec![dx_line, atr_line, tr_line])
    }
}

/// Performs the main calculation loop for the DX indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - A mutable reference to the indicator state.
/// * `inv_multiplier` - The inverse smoothing multiplier used for ATR calculation.
/// * `out_vecs` - A tuple of mutable output slices: `(dx_line, atr_line, tr_line)`.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State<Warm>,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (dx_line, atr_line, tr_line) = out_vecs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (dx, atr, tr) = TState::calc(state, inputs);
        unsafe {
            *dx_line.get_unchecked_mut(i) = dx;
        }
        if has_optional {
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, state.atr_state.wilders_state.inv_multiplier)
            );
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr
            );
        }
    }
}

pub struct Dx;

impl Indicator<INPUTS, OPTIONS> for Dx {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "dx",
        full_name: "Directional Movement Index",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["dx"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "dx",
                label: "DX",
                display_type: DisplayType::Indicator,
                outputs: &["dx"],
            },
            DisplayGroup {
                offset: None,
                id: "atr_tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["atr", "tr"],
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

        let capacity = Self::output_length(inputs[0].len(), options);
        let tr_capacity = Tr::output_length(inputs[0].len(), &[]);
        let (mut dx_line, mut atr_line, mut tr_line);
        {
            dx_line = crate::uninit_vec!(f64, capacity);
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: tr_capacity
            );
        }

        let mut state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line);
        let tr = {
            let offset = crate::slice_outputs_start!(dx_line.len(), tr_line);
            &mut tr_line[offset..]
        };
        let (high, low, close) = (
            &inputs[0][period..],
            &inputs[1][period..],
            &inputs[2][period..],
        );
        cycle(
            high,
            low,
            close,
            &mut state,
            (&mut dx_line, &mut atr_line, tr),
        );

        Ok((vec![dx_line, atr_line, tr_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::dx_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Dx {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::dx_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Dx {
    /// `DX(p)` = `(|DM+ − DM−| · 100 / (DM+ + DM−)).max(0)` — a PURE function
    /// of the two composite `Wilders` accumulators, so its recipe rebuilds the
    /// SAME `+DM`/`−DM`/accumulator subgraph as `Dm`/`Di` (dedups to one chain)
    /// and needs only high/low (`n_fields = 2`; the hand's 3rd input, `close`,
    /// feeds only its optional ATR/TR mirrors). Exact op order
    /// `(diff · 100) / sum` (left-assoc), then `Max(_,0)`; the divide emits the
    /// raw sum (hand `calc_dx` skips the `÷atr`). Publish lead = `period`.
    /// `Adx` chains a mean `Wilders` onto this node.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        let period = options[0] as u32;
        let mut r = crate::cse::Recipe::new();
        let high = r.add(crate::cse::Node::Input { stream: 0 });
        let low = r.add(crate::cse::Node::Input { stream: 1 });
        let zero = r.constant(0.0);
        let ph = r.add(crate::cse::Node::Lag { src: high, n: 1 });
        let pl = r.add(crate::cse::Node::Lag { src: low, n: 1 });
        let up = r.add(crate::cse::Node::Sub { a: high, b: ph });
        let down = r.add(crate::cse::Node::Sub { a: pl, b: low });
        let up_pos = r.add(crate::cse::Node::Max { a: up, b: zero });
        let down_pos = r.add(crate::cse::Node::Max { a: down, b: zero });
        let dp = r.add(crate::cse::Node::WhereGe {
            a: up,
            b: down,
            t: up_pos,
        });
        let dm = r.add(crate::cse::Node::WhereGe {
            a: down,
            b: up,
            t: down_pos,
        });
        let dmup = r.add(crate::cse::Node::Wilders {
            src: dp,
            period,
            composite: true,
            feed: false,
        });
        let dmdown = r.add(crate::cse::Node::Wilders {
            src: dm,
            period,
            composite: true,
            feed: false,
        });
        let dm_sub = r.add(crate::cse::Node::Sub { a: dmup, b: dmdown });
        let dm_diff = r.add(crate::cse::Node::Abs { src: dm_sub });
        let hundred = r.constant(100.0);
        let numer = r.add(crate::cse::Node::Mul {
            a: dm_diff,
            b: hundred,
        });
        let denom = r.add(crate::cse::Node::Add { a: dmup, b: dmdown });
        let quot = r.add(crate::cse::Node::Div { a: numer, b: denom });
        let dx = r.add(crate::cse::Node::Max { a: quot, b: zero });
        r.kind = "dx";
        r.options = options.to_vec();
        r.outputs.push(crate::cse::Sink::iface_lead(dx, period));
        r
    }
}
