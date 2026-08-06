//! # Ehlers TrendMode
//!
//! **Source:** John Ehlers, *Cybernetic Analysis for Stocks and Futures* (2004), Chapter 8.
//!
//! Classifies each bar as either Trend Mode or Cycle Mode by comparing the
//! current CyberCycle amplitude to its running peak envelope. When the
//! oscillator amplitude collapses to less than 20 % of its decaying peak,
//! the instrument is trending and the CyberCycle signal should be ignored;
//! otherwise the market is cycling and the CyberCycle is reliable.
//!
//! ## Algorithm
//!
//! ```text
//! Cycle = Ehlers CyberCycle oscillator (α = options[0], default 0.07)
//!
//! Peak = max(Peak[1] × 0.991, |Cycle|)   (exponential-decay amplitude latch)
//!
//! TrendMode = 1  if  Peak > 0  and  |Cycle| < 0.2 × Peak
//!           = 0  otherwise
//! ```
//!
//! ## Warmup
//!
//! `init_state` absorbs bars 0–54 (HD warmup + CyberCycle seeding + peak
//! accumulation) and produces the first output at bar 55. `min_data` = 56.
//!
//! ## Alpha / adaptive mode
//!
//! * `options[0] > 0.0` — fixed α, e.g. Ehlers' default `0.07`.
//! * `options[0] = 0.0` — **adaptive**: α is re-derived every bar from the
//!   Homodyne Discriminator's `SmoothPeriod` via `2 / (SmoothPeriod.max(3) + 1)`.
//!   The filter self-tunes to the dominant cycle; no parameter selection needed.
//!   Small extra cost vs fixed α: one `max` + one division per bar.

use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::{cybercycle, homodynediscriminator};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1; // [alpha]

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trendmode_simd::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trendmode_simd::indicator_by_options;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::trendmode_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes one asset with `N` different alpha values in parallel.
    pub use crate::indicators::simd_indicators::trendmode_simd::indicator_by_options as indicator;
}

/// Per-bar filter state for the Ehlers TrendMode.
///
/// Composes the full [`homodynediscriminator::State`] pipeline (adaptive DC period)
/// and [`cybercycle::State`] (2-pole high-pass oscillator), then extends them with
/// a decaying peak-amplitude latch.
///
/// **Warmup:** after [`init_state`](State::init_state) completes all ring buffers
/// are full and the IIR feedback is seeded. The hot path (`calc_unchecked`)
/// operates unconditionally.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// Embedded Homodyne Discriminator — provides `SmoothPeriod` (DC) per bar.
    pub hd: homodynediscriminator::State<S>,
    /// Embedded CyberCycle oscillator — produces `Cycle` per bar.
    pub cc: cybercycle::State<S>,
    /// Running peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
    pub pk: f64,
    /// Alpha value used to construct this state. `0.0` = adaptive.
    pub alpha: f64,
    pub(crate) is_adaptive: bool
}

impl State<Cold> {
    /// Creates a zeroed state ready for the first bar.
    pub fn new(alpha: f64) -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            cc: if alpha > 0.0 {
                cybercycle::State::new(alpha)
            } else {
                cybercycle::State::default()
            },
            pk: 0.0,
            alpha,
            is_adaptive: alpha == 0.0,
        }
    }
    pub fn into_full(self) -> State<Warm> {
        State {
            hd: self.hd.into_full(),
            cc: self.cc.into_full(),
            pk: self.pk,
            alpha: self.alpha,
            is_adaptive: self.is_adaptive,
        }
    }
    /// Builds a warmed-up state by seeding the HD and CC pipelines over 55
    /// bars, then processes bar 55 (the first valid output).
    ///
    /// **Three phases:**
    /// 1. Bars 0–5:  CC seeding (second-difference formula) + `hd.calc()` (safe).
    /// 2. Bars 6–21: `hd.calc()` (safe) + `cc.calc_unchecked()` + peak tracking.
    /// 3. Bars 22–54: `hd.calc_unchecked()` + `cc.calc_unchecked()` + peak tracking.
    ///
    /// Writes the first output values to the respective output slices at index 0.
    /// Pass empty slices (`&mut []`) for any optional output that is not needed.
    pub fn init_state(
        real: &[f64],
        alpha: f64, // 0.0 = adaptive; (0,1) = fixed
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

        // ── Phase 2: bars 6–21 — HD safe + CC unchecked + peak tracking ──────
        for i in 6..22 {
            state.hd.calc(real[i]);
            if alpha == 0.0 {
                let a = cybercycle::adaptive_alpha(state.hd.smooth_period);
                let (coef, d1, d2) = cybercycle::multiplier(a);
                state.cc.coef = coef;
                state.cc.d1 = d1;
                state.cc.d2 = d2;
            }
            let cycle =state.cc.calc(real[i]);
            state.pk = (state.pk * 0.991).max(cycle.abs());
        }

        // ── Phase 3: bars 22–54 — both unchecked + peak tracking ─────────────
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
        }

        // ── Bar 55: first valid output ────────────────────────────────────────
        let trendmode = if state.is_adaptive {
            state.calc_adaptive(real[55])
        } else {
           state.calc(real[55])
        };
        trendmode_line[0] = trendmode;
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
            alpha,
            is_adaptive: state.is_adaptive
        }
    }
}

impl<S> State<S>
where
    for<'a> homodynediscriminator::State<S>: TState<Inputs<'a> = f64>,
    for<'a> cybercycle::State<S>: TState<Inputs<'a> = f64, Outputs = f64>,
{
    #[inline(always)]
    pub fn calc_adaptive(&mut self, price: f64) -> f64 {
        self.hd.calc(price);
        let alpha = cybercycle::adaptive_alpha(self.hd.smooth_period);
        let (coef, d1, d2) = cybercycle::multiplier(alpha);
        self.cc.coef = coef;
        self.cc.d1 = d1;
        self.cc.d2 = d2;
        let cycle = self.cc.calc(price);
        self.pk = (self.pk * 0.991).max(cycle.abs());
        if self.pk > 0.0 && cycle.abs() < 0.2 * self.pk {
            1.0
        } else {
            0.0
        }
    }
    #[inline(always)]
    fn calc_dispatch(&mut self, price: f64) -> f64 {
        if self.is_adaptive {
            self.calc_adaptive(price)
        } else {
            self.hd.calc(price);
            let cycle = self.cc.calc(price);
            self.pk = (self.pk * 0.991).max(cycle.abs());
            if self.pk > 0.0 && cycle.abs() < 0.2 * self.pk {
                1.0
            } else {
                0.0
            }
        }
    }
}


impl Default for State<Cold> {
    fn default() -> Self {
        Self::new(0.0)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64; // trendmode (1.0 or 0.0)
    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        self.calc_dispatch(price)
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = f64; // trendmode (1.0 or 0.0)

    fn calc<'a>(&mut self, price: f64) -> f64 {
        self.calc_dispatch(price)
    }
}

/// `IndicatorState` is the complete self-contained state — coefficients live on `cc` alongside filter history.
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
        let mut trendmode_line = crate::uninit_vec!(f64, n);
        let (mut cycle_line, mut peak_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            cycle_line: n,
            peak_line: n
        );

        run_trendmode(
            real,
            self,
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        Ok(vec![trendmode_line, cycle_line, peak_line])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Unit struct that implements [`Indicator`] for the Ehlers TrendMode.
pub struct TrendMode;

impl Indicator<INPUTS, OPTIONS> for TrendMode {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "trendmode",
        indicator_type: IndicatorType::Trend,
        full_name: "Ehlers TrendMode",
        inputs: &["real"],
        options: &["alpha"],
        outputs: &["trendmode"],
        optional_outputs: &["cycle", "peak"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "trendmode",
                label: "Ehlers TrendMode",
                display_type: DisplayType::Indicator,
                outputs: &["trendmode"],
            },
            DisplayGroup {
                offset: None,
                id: "trendmode_cycle",
                label: "TrendMode CyberCycle",
                display_type: DisplayType::Indicator,
                outputs: &["cycle"],
            },
            DisplayGroup {
                offset: None,
                id: "trendmode_peak",
                label: "TrendMode Peak",
                display_type: DisplayType::Indicator,
                outputs: &["peak"],
            },
        ],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        56
    }

    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len.saturating_sub(55)
    }

    /// Calculates the Ehlers TrendMode over the full input dataset.
    ///
    /// # Inputs
    ///
    /// * `inputs[0]` — close (or HLC/3) price series
    ///
    /// # Options
    ///
    /// * `options[0]` — `alpha` ∈ [0, 1). `0` = adaptive (derived from DC period each bar).
    ///   Ehlers' default fixed value is `0.07`.
    ///
    /// # Outputs
    ///
    /// * `outputs[0]` — `trendmode`: `1.0` = Trend Mode, `0.0` = Cycle Mode
    /// * `outputs[1]` — `cycle`:    CyberCycle oscillator (optional; empty unless requested)
    /// * `outputs[2]` — `peak`:     decaying amplitude peak (optional; empty unless requested)
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;

        let alpha = options[0];
        let real = inputs[0];
        let capacity = Self::output_length(real.len(), options);

        let mut trendmode_line = crate::uninit_vec!(f64, capacity);
        let (mut cycle_line, mut peak_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            cycle_line: capacity,
            peak_line: capacity
        );

        // init_state seeds bars 0–54 and processes bar 55 (output index 0).
        let mut state = State::init_state(
            real,
            alpha,
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        let (cycle_tail, peak_tail) = {
            let o = crate::slice_outputs_start!(capacity - 1, cycle_line, peak_line);
            (&mut cycle_line[o.0..], &mut peak_line[o.1..])
        };

        // Process bars 56..n (output indices 1..capacity).
        run_trendmode(
            &real[Self::min_data(options)..],
            &mut state,
            &mut trendmode_line[1..],
            cycle_tail,
            peak_tail,
        );

        Ok((vec![trendmode_line, cycle_line, peak_line], state))
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

/// Shared hot loop used by both `indicator` and `batch_indicator`.
///
/// All HD and CC ring buffers must be full on entry (guaranteed after
/// `init_state`). Writes `trendmode` for every bar, and optionally `cycle` and
/// `peak`.
fn run_trendmode(
    real: &[f64],
    state: &mut State<Warm>,
    trendmode_line: &mut [f64],
    cycle_line: &mut [f64],
    peak_line: &mut [f64],
) {
    let (has_optional, want_cycle, want_peak) = crate::calc_want_flags!(cycle_line, peak_line);
    if state.alpha == 0.0 {
        for i in 0..real.len() {
            let trendmode = state.calc_adaptive(unsafe { *real.get_unchecked(i) });
            unsafe {
                *trendmode_line.get_unchecked_mut(i) = trendmode;
            }
            if has_optional {
                crate::store_optional_outputs!(i,
                    want_cycle, cycle_line => state.cc.cycle_prev,
                    want_peak,  peak_line  => state.pk
                );
            }
        }
    } else {
        for i in 0..real.len() {
            let trendmode = state.calc(unsafe { *real.get_unchecked(i) });
            unsafe {
                *trendmode_line.get_unchecked_mut(i) = trendmode;
            }
            if has_optional {
                crate::store_optional_outputs!(i,
                    want_cycle, cycle_line => state.cc.cycle_prev,
                    want_peak,  peak_line  => state.pk
                );
            }
        }
    }
}
