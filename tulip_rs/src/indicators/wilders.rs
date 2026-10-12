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

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub wilders: f64,
    pub multiplier: f64,
    pub inv_multiplier: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(wilders: f64, period: usize) -> Self {
        let (multiplier, inv_multiplier) = multiplier(period);
        Self {
            multiplier,
            inv_multiplier,
            wilders,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            wilders: self.wilders,
            multiplier: self.multiplier,
            inv_multiplier: self.inv_multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let wilders = real.iter().take(period).sum::<f64>() / period as f64;
        let multipliers = multiplier(period);
        State {
            wilders,
            multiplier: multipliers.0,
            inv_multiplier: multipliers.1,
            state: std::marker::PhantomData,
        }
    }
}
impl State<Warm> {
    pub fn partial_calc(&mut self, value: f64) -> f64 {
        //prev_wilders * multiplier + value
        self.wilders = self.wilders.mul_add(self.multiplier, value);
        self.wilders
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real = inputs[0];
        let mut wilders_line = crate::uninit_vec!(f64, real.len());
        for i in 0..real.len() {
            unsafe { *wilders_line.get_unchecked_mut(i) = self.calc(*real.get_unchecked(i)) }
        }

        Ok(vec![wilders_line])
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Wilders {
    /// The graph is leaf data — `crate::cse::recipes::wilders` (one
    /// mean-mode decayed-average node over `Input{0}`, publish lead
    /// `period`).
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::wilders(options)
    }
}

/// Rebuild the hand `wilders::State<Warm>` from the carried decayed-average
/// value (`get_states`).
pub(crate) fn restore_state(v: Vec<crate::cse::ChainState>) -> crate::cse::IndicatorStateBox {
    let [crate::cse::ChainState::Wilders { value, period }] = v.as_slice() else {
        return crate::cse::IndicatorStateBox::Other;
    };
    crate::cse::IndicatorStateBox::Wilders(State::new(*value, *period as usize).into_warm())
}

impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        self.wilders = self
            .wilders
            .mul_add(self.multiplier, value * self.inv_multiplier);
        self.wilders
    }
}

/// `((p−1)/p, 1/p)` — the Wilders step pair, in the leaf vocabulary
/// (`tulip_rs_shared::cse::multipliers`); re-exported here to keep every
/// consumer's `wilders::multiplier` path.
pub use tulip_rs_shared::cse::multipliers::wilders_multiplier as multiplier;

pub struct Wilders;

impl Indicator<INPUTS, OPTIONS> for Wilders {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "wilders",
        full_name: "Wilder's Smoothing",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["wilders"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "wilders",
            label: "WILDERS",
            display_type: DisplayType::Overlay,
            outputs: &["wilders"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        let mut wilders_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::init_state(inputs[0], period);

        let real = &inputs[0][period..];
        for i in 0..real.len() {
            unsafe {
                *wilders_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
            }
        }

        Ok((vec![wilders_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wilders_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Wilders {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wilders_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
