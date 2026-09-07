//! # Ehlers CyberCycle Fisher
//!
//! **Source:** John Ehlers, *Cybernetic Analysis for Stocks and Futures* (2004), Chapter 8.
//!
//! Applies a Fisher Transform to the normalised Ehlers CyberCycle oscillator,
//! converting the near-Gaussian oscillator amplitude into a near-Gaussian
//! probability distribution with sharper turning-point signals. The Fisher line
//! is the primary output; its one-bar lag serves as the signal (trigger) line.
//!
//! ## Algorithm
//!
//! ```text
//! Cycle  = Ehlers CyberCycle (α = options[0], default 0.07)
//!
//! Peak   = max(Peak[1] × 0.991, |Cycle|)         (decaying amplitude envelope)
//!
//! Value  = Cycle / Peak   (0 when Peak = 0)
//!
//! Value1 = clamp(0.65 × Value1[1] + 0.35 × Value, −0.999, 0.999)
//!
//! Fisher = 0.5 × ln( (1 + Value1) / (1 − Value1) )
//!
//! Signal = Fisher[1]      (one-bar lag — the trigger line)
//! ```
//!
//! ## Warmup
//!
//! `init_state` absorbs bars 0–54 (HD warmup + CyberCycle seeding + peak /
//! value accumulation) and produces the first output at bar 55.  `min_data` = 56.
//!
//! ## Alpha / adaptive mode
//!
//! * `options[0] > 0.0` — fixed α, e.g. Ehlers' default `0.07`.
//! * `options[0] = 0.0` — **adaptive**: α is re-derived every bar from the
//!   Homodyne Discriminator's `SmoothPeriod` via `2 / (SmoothPeriod.max(3) + 1)`.
//!   The filter self-tunes to the dominant cycle; no parameter selection needed.
//!   Small extra cost vs fixed α: one `max` + one division per bar.

use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::{cybercycle, homodynediscriminator};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1; // [alpha]

/// Per-bar filter state for the Ehlers CyberCycle Fisher.
///
/// Composes a [`homodynediscriminator::State`] and a [`cybercycle::State`],
/// then extends them with a decaying peak-amplitude latch, a smoothed normalised
/// value (`val1`), and the previous Fisher value (`fish`).
///
/// Implements [`Deref`]/[`DerefMut`] targeting [`cybercycle::State`] so CC fields
/// (`coef`, `d1`, `d2`, `cycle_prev`, …) are accessible without `.cc.` indirection.
///
/// The `alpha` field drives the fixed/adaptive dispatch on every bar:
/// - `alpha = 0.0` — adaptive: CC coefficients are re-derived each bar from
///   the HD's `smooth_period` via [`cybercycle::adaptive_alpha`].
/// - `alpha ∈ (0, 1)` — fixed: coefficients are set once in [`State::new`].
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// Embedded Homodyne Discriminator — provides `SmoothPeriod` (DC) per bar.
    pub hd: homodynediscriminator::State<S>,
    /// Embedded CyberCycle oscillator — `coef/d1/d2` live here; no duplication.
    pub cc: cybercycle::State<S>,
    /// Running peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
    pub pk: f64,
    /// Smoothed normalised value: `clamp(0.65×val1[1] + 0.35×(Cycle/Peak), ±0.999)`.
    pub val1: f64,
    /// Fisher value from the previous bar — emitted as the signal line.
    pub fish: f64,
    /// `0.0` = adaptive; `(0, 1)` = fixed. Stored for `TState` dispatch and serde.
    pub alpha: f64,
    pub is_adaptive: bool,
}

impl<S> Deref for State<S> {
    type Target = cybercycle::State<S>;
    fn deref(&self) -> &Self::Target {
        &self.cc
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cc
    }
}

impl State<Cold> {
    /// Creates a zeroed filter state with `alpha` embedded.
    ///
    /// For fixed alpha: CC coefficients (`coef/d1/d2`) are precomputed from `alpha`.
    /// For adaptive (`alpha = 0.0`): CC coefficients are zeroed — they are updated
    /// every bar from the HD's `smooth_period` before `cc.calc_unchecked` is called.
    pub fn new(alpha: f64) -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            cc: if alpha > 0.0 {
                cybercycle::State::new(alpha)
            } else {
                // Derived Default gives coef = d1 = d2 = 0.0 — overwritten each bar.
                cybercycle::State::default()
            },
            pk: 0.0,
            val1: 0.0,
            fish: 0.0,
            alpha,
            is_adaptive: alpha == 0.0,
        }
    }

    /// Builds a warmed-up state by seeding the HD and CC pipelines over 55 bars,
    /// then processes bar 55 (the first valid output).
    ///
    /// **Three phases:**
    /// 1. Bars 0–5:  CC seeding (second-difference) + `hd.calc()`.
    /// 2. Bars 6–21: `hd.calc()` + `cc.calc_unchecked()` + Fisher tracking.
    /// 3. Bars 22–54: `hd.calc_unchecked()` + `cc.calc_unchecked()` + Fisher tracking.
    ///
    /// Pass empty slices (`&mut []`) for any optional output that is not needed.
    pub fn init_state(
        real: &[f64],
        alpha: f64,
        fisher_line: &mut [f64],
        signal_line: &mut [f64],
        trendmode_line: &mut [f64],
        cycle_line: &mut [f64],
        peak_line: &mut [f64],
    ) -> State<Warm> {
        let mut state = Self::new(alpha);

        // ── Phase 1: bars 0–5 — CC seeding + HD warmup ───────────────────────
        for i in 0..6 {
            state.cc.price_buf.push(real[i]);
            if state.cc.price_buf.len() >= 4 {
                let ab = 2.0_f64.mul_add(state.cc.price_buf[1], state.cc.price_buf[0]);
                let cd = 2.0_f64.mul_add(state.cc.price_buf[2], state.cc.price_buf[3]);
                state.cc.smooth_buf.push((ab + cd) * (1.0 / 6.0));
            }
            if state.cc.price_buf.len() >= 3 {
                let seed = (state.cc.price_buf[0] - 2.0 * state.cc.price_buf[1]
                    + state.cc.price_buf[2])
                    / 4.0;
                state.cc.cycle_prev2 = state.cc.cycle_prev;
                state.cc.cycle_prev = seed;
            }
            state.hd.calc(real[i]);
        }

        // ── Phase 2: bars 6–21 — HD safe + CC unchecked + Fisher tracking ────
        for i in 6..22 {
            state.hd.calc(real[i]);
            if alpha == 0.0 {
                let a = cybercycle::adaptive_alpha(state.hd.smooth_period);
                let (coef, d1, d2) = cybercycle::multiplier(a);
                state.cc.coef = coef;
                state.cc.d1 = d1;
                state.cc.d2 = d2;
            }
            let cycle = state.cc.calc(real[i]);
            state.pk = (state.pk * 0.991).max(cycle.abs());
            let value = if state.pk > 0.0 {
                cycle / state.pk
            } else {
                0.0
            };
            state.val1 = (0.65 * state.val1 + 0.35 * value).clamp(-0.999, 0.999);
            let ln_arg = (1.0 + state.val1) / (1.0 - state.val1);
            state.fish = 0.5 * ln_arg.ln();
        }

        // ── Phase 3: bars 22–54 — both unchecked + Fisher tracking ───────────
        for i in 22..55 {
            state.hd.calc(real[i]);
            if alpha == 0.0 {
                let a = cybercycle::adaptive_alpha(state.hd.smooth_period);
                let (coef, d1, d2) = cybercycle::multiplier(a);
                state.cc.coef = coef;
                state.cc.d1 = d1;
                state.cc.d2 = d2;
            }
            let cycle = state.cc.calc(real[i]);
            state.pk = (state.pk * 0.991).max(cycle.abs());
            let value = if state.pk > 0.0 {
                cycle / state.pk
            } else {
                0.0
            };
            state.val1 = (0.65 * state.val1 + 0.35 * value).clamp(-0.999, 0.999);
            let ln_arg = (1.0 + state.val1) / (1.0 - state.val1);
            state.fish = 0.5 * ln_arg.ln();
        }

        // ── Bar 55: first valid output ────────────────────────────────────────
        let (fisher, signal) = if alpha == 0.0 {
            state.calc_adaptive(real[55])
        } else {
            state.calc(real[55])
        };
        fisher_line[0] = fisher;
        signal_line[0] = signal;
        if !trendmode_line.is_empty() {
            let cycle = state.cc.cycle_prev;
            trendmode_line[0] = if state.pk > 0.0 && cycle.abs() < 0.2 * state.pk {
                1.0
            } else {
                0.0
            };
        }
        if !cycle_line.is_empty() {
            cycle_line[0] = state.cc.cycle_prev;
        }
        if !peak_line.is_empty() {
            peak_line[0] = state.pk;
        }
        State {
            hd: state.hd.into_full(),
            cc: state.cc.into_full(),
            pk: state.pk,
            val1: state.val1,
            fish: state.fish,
            alpha: state.alpha,
            is_adaptive: state.is_adaptive,
        }
    }
}
impl<S> State<S>
where
    for<'a> homodynediscriminator::State<S>: TState<Inputs<'a> = f64>,
    for<'a> cybercycle::State<S>: TState<Inputs<'a> = f64, Outputs = f64>,
{
    #[inline(always)]
    pub fn calc_adaptive(&mut self, price: f64) -> (f64, f64) {
        self.hd.calc(price);
        let alpha = cybercycle::adaptive_alpha(self.hd.smooth_period);
        let (coef, d1, d2) = cybercycle::multiplier(alpha);
        self.cc.coef = coef;
        self.cc.d1 = d1;
        self.cc.d2 = d2;
        let cycle = self.cc.calc(price);
        self.pk = (self.pk * 0.991).max(cycle.abs());
        let value = if self.pk > 0.0 { cycle / self.pk } else { 0.0 };
        self.val1 = (0.65 * self.val1 + 0.35 * value).clamp(-0.999, 0.999);
        let ln_arg = (1.0 + self.val1) / (1.0 - self.val1);
        let fish = 0.5 * ln_arg.ln();
        let signal = self.fish;
        self.fish = fish;
        (fish, signal)
    }
    #[inline(always)]
    pub fn calc_dispatch(&mut self, price: f64) -> (f64, f64) {
        if self.is_adaptive {
            self.calc_adaptive(price)
        } else {
            calc(self, price)
        }
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, price: f64) -> Self::Outputs {
        self.calc_dispatch(price)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, price: f64) -> Self::Outputs {
        self.calc_dispatch(price)
    }
}

impl Default for State<Cold> {
    fn default() -> Self {
        Self::new(0.0)
    }
}
#[inline(always)]
fn calc<S>(state: &mut State<S>, price: f64) -> (f64, f64)
where
    for<'a> homodynediscriminator::State<S>: TState<Inputs<'a> = f64>,
    for<'a> cybercycle::State<S>: TState<Inputs<'a> = f64, Outputs = f64>,
{
    state.hd.calc(price);
    let cycle = state.cc.calc(price);
    state.pk = (state.pk * 0.991).max(cycle.abs());
    let value = if state.pk > 0.0 {
        cycle / state.pk
    } else {
        0.0
    };
    state.val1 = (0.65 * state.val1 + 0.35 * value).clamp(-0.999, 0.999);
    let ln_arg = (1.0 + state.val1) / (1.0 - state.val1);
    let fish = 0.5 * ln_arg.ln();
    let signal = state.fish;
    state.fish = fish;
    (fish, signal)
}
/// `IndicatorState` is `State` directly — `alpha` and `cc` coefficients live
/// inside the state; no separate wrapper or duplicated multipliers field needed.
pub type IndicatorState = State<Warm>;

impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real = inputs[0];
        let n = real.len();
        let mut fisher_line = crate::uninit_vec!(f64, n);
        let mut signal_line = crate::uninit_vec!(f64, n);
        let (mut trendmode_line, mut cycle_line, mut peak_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            trendmode_line: n,
            cycle_line: n,
            peak_line: n
        );

        run_ccfisher(
            real,
            self,
            &mut fisher_line,
            &mut signal_line,
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        Ok(vec![
            fisher_line,
            signal_line,
            trendmode_line,
            cycle_line,
            peak_line,
        ])
    }
}

/// Validates `alpha`.
///
/// * `0.0` — adaptive (derived from `SmoothPeriod` each bar via the embedded HD).
/// * `(0.0, 1.0)` — fixed user-supplied alpha. Ehlers' default is `0.07`.
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 0.0 || options[0] >= 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Shared hot loop used by both `CcFisher::indicator` and `batch_indicator`.
///
/// All HD and CC ring buffers must be full on entry (guaranteed after `init_state`).
/// Dispatches to adaptive or fixed path via `state.alpha`.
fn run_ccfisher(
    real: &[f64],
    state: &mut State<Warm>,
    fisher_line: &mut [f64],
    signal_line: &mut [f64],
    trendmode_line: &mut [f64],
    cycle_line: &mut [f64],
    peak_line: &mut [f64],
) {
    let (has_optional, want_trendmode, want_cycle, want_peak) =
        crate::calc_want_flags!(trendmode_line, cycle_line, peak_line);

    if state.alpha == 0.0 {
        for i in 0..real.len() {
            let (fisher, signal) = state.calc_adaptive(unsafe { *real.get_unchecked(i) });
            unsafe {
                *fisher_line.get_unchecked_mut(i) = fisher;
                *signal_line.get_unchecked_mut(i) = signal;
            }
            if has_optional {
                let cycle_val = state.cc.cycle_prev;
                let pk_val = state.pk;
                let tm = if pk_val > 0.0 && cycle_val.abs() < 0.2 * pk_val {
                    1.0_f64
                } else {
                    0.0_f64
                };
                crate::store_optional_outputs!(i,
                    want_trendmode, trendmode_line => tm,
                    want_cycle,     cycle_line     => cycle_val,
                    want_peak,      peak_line      => pk_val
                );
            }
        }
    } else {
        for i in 0..real.len() {
            let (fisher, signal) = state.calc(unsafe { *real.get_unchecked(i) });
            unsafe {
                *fisher_line.get_unchecked_mut(i) = fisher;
                *signal_line.get_unchecked_mut(i) = signal;
            }
            if has_optional {
                let cycle_val = state.cc.cycle_prev;
                let pk_val = state.pk;
                let tm = if pk_val > 0.0 && cycle_val.abs() < 0.2 * pk_val {
                    1.0_f64
                } else {
                    0.0_f64
                };
                crate::store_optional_outputs!(i,
                    want_trendmode, trendmode_line => tm,
                    want_cycle,     cycle_line     => cycle_val,
                    want_peak,      peak_line      => pk_val
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Indicator trait
// ─────────────────────────────────────────────────────────────────────────────

/// Zero-sized marker type for the `Indicator` trait impl.
pub struct CcFisher;

impl Indicator<INPUTS, OPTIONS> for CcFisher {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ccfisher",
        indicator_type: IndicatorType::Momentum,
        full_name: "Cyber Cycle Fisher",
        inputs: &["real"],
        options: &["alpha"],
        outputs: &["fisher", "signal"],
        optional_outputs: &["trendmode", "cycle", "peak"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "ccfisher",
                label: "Ehlers CyberCycle Fisher",
                display_type: DisplayType::Indicator,
                outputs: &["fisher", "signal"],
            },
            DisplayGroup {
                offset: None,
                id: "ccfisher_trendmode",
                label: "CCFisher TrendMode",
                display_type: DisplayType::Indicator,
                outputs: &["trendmode"],
            },
            DisplayGroup {
                offset: None,
                id: "ccfisher_cycle",
                label: "CCFisher CyberCycle",
                display_type: DisplayType::Indicator,
                outputs: &["cycle"],
            },
            DisplayGroup {
                offset: None,
                id: "ccfisher_peak",
                label: "CCFisher Peak",
                display_type: DisplayType::Indicator,
                outputs: &["peak"],
            },
        ],
    };

    /// Fixed at 56 bars — 55-bar warmup (HD + CC seeding + Fisher), first output at bar 55.
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        56
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;

        let alpha = options[0];
        let real = inputs[0];
        let n = real.len();
        let capacity = Self::output_length(n, options);

        let mut fisher_line = crate::uninit_vec!(f64, capacity);
        let mut signal_line = crate::uninit_vec!(f64, capacity);
        let (mut trendmode_line, mut cycle_line, mut peak_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            trendmode_line: capacity,
            cycle_line: capacity,
            peak_line: capacity
        );

        // init_state seeds bars 0–54 and processes bar 55 (output index 0).
        let mut state = State::init_state(
            real,
            alpha,
            &mut fisher_line,
            &mut signal_line,
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        let (trendmode_tail, cycle_tail, peak_tail) = {
            let o =
                crate::slice_outputs_start!(capacity - 1, trendmode_line, cycle_line, peak_line);
            (
                &mut trendmode_line[o.0..],
                &mut cycle_line[o.1..],
                &mut peak_line[o.2..],
            )
        };

        // Process bars 56..n (output indices 1..capacity).
        run_ccfisher(
            &real[Self::min_data(options)..],
            &mut state,
            &mut fisher_line[1..],
            &mut signal_line[1..],
            trendmode_tail,
            cycle_tail,
            peak_tail,
        );

        Ok((
            vec![
                fisher_line,
                signal_line,
                trendmode_line,
                cycle_line,
                peak_line,
            ],
            state,
        ))
    }
    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ccfisher_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for CcFisher {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ccfisher_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
