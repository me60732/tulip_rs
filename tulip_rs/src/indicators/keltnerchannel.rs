use crate::common::validate_inputs;
use crate::cse::{Planable, Recipe};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::{
    atr::{multiplier as atr_multiplier, Atr, State as AtrState},
    ema::{multiplier as ema_multiplier, Ema, State as EmaState},
    tr::Tr,
};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub atr_state: AtrState<S>,
    pub ema_state: EmaState<S>,
    pub step: f64,
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> (f64, f64, f64, f64, f64) {
        let (atr, tr) = self.atr_state.calc((high, low, close));
        let ema = self.ema_state.calc(close);

        let per = atr * self.step;
        let upper = ema + per;
        let lower = ema - per;

        (lower, ema, upper, atr, tr)
    }
}
impl State<Cold> {
    /// Initialises the Keltner Channel state from the first `period` bars.
    ///
    /// Seeds the ATR with the simple-average true range over `[0, period)` and
    /// seeds the EMA with the exponentially-smoothed close over the same window.
    /// If `tr_line` is non-empty the raw true-range values for bars `[1, period)` are
    /// written into it (index 0 = bar 1).
    ///
    /// # Arguments
    ///
    /// * `high` - High prices; must contain at least `period` elements.
    /// * `low` - Low prices; must contain at least `period` elements.
    /// * `close` - Close prices; must contain at least `period` elements.
    /// * `period` - Lookback period for ATR and EMA initialisation.
    /// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
    /// * `tr_line` - Optional output buffer for raw true-range values written during warm-up.
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        step: f64,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, false);
        let ema_state = EmaState::init_state(close, period);

        State {
            atr_state,
            ema_state,
            step,
        }
    }
}
pub type IndicatorState = State<Warm>;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let [high, low, close] = inputs;
        let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: len,
                    tr_line: len
                ),
            )
        };

        cycle(
            (high, low, close),
            (&mut lower_band, &mut middle_band, &mut upper_band),
            self,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![lower_band, middle_band, upper_band, atr_line, tr_line])
    }
}

/// Validates Keltner Channel options.
///
/// # Errors
///
/// Returns `Err(IndicatorError::InvalidOptions)` if `period < 1` or `step ≤ 0`.
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Performs the main calculation loop for the Keltner Channel indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices (starting at the first output bar).
/// * `step` - The ATR multiplier controlling channel width.
/// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
/// * `outputs` - A tuple of mutable slices for storing the `(lower, middle, upper)` channel bands.
/// * `state` - A mutable reference to the current indicator state.
/// * `optional_outputs` - A tuple of mutable slices for optional `(atr, tr)` outputs.
fn cycle(
    (high, low, close): (&[f64], &[f64], &[f64]),
    (lower_band, middle_band, upper_band): (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State<Warm>,
    (atr_line, tr_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);
    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (lower, middle, upper, atr, tr) = state.calc(inputs);

        unsafe {
            *middle_band.get_unchecked_mut(i) = middle;
            *upper_band.get_unchecked_mut(i) = upper;
            *lower_band.get_unchecked_mut(i) = lower;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr
            );
        }
    }
}

/// Returns the precomputed smoothing constants for the Keltner Channel.
///
/// # Returns
///
/// A tuple `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))` where the first pair
/// contains Wilder ATR smoothing constants and the second pair contains EMA smoothing constants.
pub fn multiplier(period: usize) -> ((f64, f64), (f64, f64)) {
    (atr_multiplier(period), ema_multiplier(period))
}

pub struct KeltnerChannel;

impl Indicator<INPUTS, OPTIONS> for KeltnerChannel {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "keltnerchannel",
        full_name: "Keltner Channel",
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &["period", "step"],
        outputs: &["lower", "middle", "upper"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "keltnerchannel",
                label: "Keltner Channel",
                display_type: DisplayType::Overlay,
                outputs: &["lower", "middle", "upper"],
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
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let step = options[1];
        let [high, low, close] = inputs;

        validate_inputs(inputs, Self::min_data(options))?;

        let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: Tr::output_length(len, &[])
                ),
            )
        };

        let mut state = State::init_state(high, low, close, period, step, &mut tr_line);
        let (inputs, tr) = {
            let tr_offset = crate::slice_outputs_start!(middle_band.len(), tr_line);
            (
                (&high[period..], &low[period..], &close[period..]),
                &mut tr_line[tr_offset..],
            )
        };
        cycle(
            inputs,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut state,
            (&mut atr_line, tr),
        );

        Ok((
            vec![lower_band, middle_band, upper_band, atr_line, tr_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for KeltnerChannel {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

impl Planable for KeltnerChannel {
    /// The graph is leaf data — `crate::cse::recipes::keltnerchannel`:
    /// `Ema{close, period}` as the middle, ± `Mul{atr, step}` for the bands,
    /// with `atr` from the shared `atr_nodes` walk. Three PRIMARY rows
    /// (lower, middle, upper) at lead `period`, plus the `atr` and `tr`
    /// optionals. `ema_atr_keltner_shared_p` interns the whole ATR chain onto
    /// `Atr` — legal only because the hand stores the band half uncorrected.
    fn recipe(options: &[f64]) -> Recipe {
        crate::cse::recipes::keltnerchannel(options)
    }
}

impl crate::cse::Restorable<INPUTS, OPTIONS> for KeltnerChannel {
    const KIND: &'static str = "keltnerchannel";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        Self::restore_state_with(v, windows, &[])
    }

    fn restore_state_with(
        v: Vec<crate::cse::ChainState>,
        windows: &[&[f64]],
        options: &[f64],
    ) -> Option<Self::IndicatorState> {
        // Qualified, not imported: `ChainState::Ema` would otherwise shadow the
        // `ema::Ema` indicator type this function calls through.
        use crate::cse::ChainState;
        // COMPOSED, not reimplemented: the state is `atr_state` + `ema_state`, and
        // each is rebuilt by that indicator's OWN restorer, so there is one ATR/EMA
        // reconstruction in the crate rather than three. `Atr::restore_state` wants
        // its `[Wilders, Mirror]` pair in the order `recipes::atr` declares.
        //
        // Two bugs this replaces, both visible only because the probe runs
        // non-default options:
        //   * `step` was hard-coded `2.0` behind the comment "fallback when recipe
        //     tail does not carry `step` yet" — the tail now carries everything the
        //     state needs, and `step` comes from the request;
        //   * `period` was `windows[0].len() - 1`, i.e. the length of whatever the
        //     router handed in stood in for the parameter (the `mom` class).
        //     The periods are read back out of the chains instead, which is what
        //     makes a mismatched request impossible to answer.
        let [ChainState::Wilders {
            value: atr,
            period: atr_period,
        }, ChainState::Ema {
            value: ema,
            period: ema_period,
        }] = v.as_slice()
        else {
            return None;
        };
        let [high, low, close] = windows else {
            return None;
        };
        let step = *options.get(1)?;
        if high.len() < 2 || low.len() != high.len() || close.len() != high.len() {
            return None;
        }
        // `tr_state`'s `prev_close` for the NEXT bar is the seam bar's close — a raw
        // price, so the column holds it bit-for-bit and no `Mirror` node is needed.
        let prev_close = *close.last()?;
        let atr_state = <Atr as crate::cse::Restorable<3, 1>>::restore_state(
            vec![
                ChainState::Wilders {
                    value: *atr,
                    period: *atr_period,
                },
                ChainState::Mirror { value: prev_close },
            ],
            &[],
        )?;
        let ema_state = <Ema as crate::cse::Restorable<1, 1>>::restore_state(
            vec![ChainState::Ema {
                value: *ema,
                period: *ema_period,
            }],
            &[],
        )?;
        Some(State {
            atr_state,
            ema_state,
            step,
        })
    }
}
