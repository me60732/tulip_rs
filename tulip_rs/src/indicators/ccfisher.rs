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
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{cybercycle, homodynediscriminator};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1; // [alpha]

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ccfisher_simd::indicator_by_assets;

/// SIMD-parallel variant that processes one asset with `N` different alpha values.
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ccfisher_simd::indicator_by_options;

/// Module alias exposing `indicator_by_assets` as `indicator`.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::ccfisher_simd::indicator_by_assets as indicator;
}

/// Module alias exposing `indicator_by_options` as `indicator`.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes one asset with `N` different alpha values in parallel.
    pub use crate::indicators::simd_indicators::ccfisher_simd::indicator_by_options as indicator;
}

/// Metadata for the Ehlers CyberCycle Fisher indicator.
pub const INFO: Info = Info {
    name: "ccfisher",
    indicator_type: IndicatorType::Cycle,
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

/// Per-bar filter state for the Ehlers CyberCycle Fisher.
///
/// Composes the full [`homodynediscriminator::State`] pipeline (adaptive DC period)
/// and [`cybercycle::State`] (2-pole high-pass oscillator), then extends them with
/// a decaying peak-amplitude latch, a smoothed normalised value (`val1`), and the
/// previous Fisher value (`fish`) used as the signal line.
///
/// **Warmup:** after [`init_state`](State::init_state) completes, all ring buffers
/// are full and the IIR feedback is seeded.  The hot path (`calc_unchecked`)
/// operates unconditionally.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// Embedded Homodyne Discriminator — provides `SmoothPeriod` (DC) per bar.
    pub hd: homodynediscriminator::State,
    /// Embedded CyberCycle oscillator — produces `Cycle` per bar.
    pub cc: cybercycle::State,
    /// Running peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
    pub pk: f64,
    /// Smoothed normalised value: `clamp(0.65×val1[1] + 0.35×(Cycle/Peak), ±0.999)`.
    pub val1: f64,
    /// Fisher value from the previous bar — emitted as the signal (trigger) line.
    pub fish: f64,
}

impl State {
    /// Creates a zeroed state ready for the first bar.
    pub fn new() -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            cc: cybercycle::State::new(),
            pk: 0.0,
            val1: 0.0,
            fish: 0.0,
        }
    }

    /// Builds a warmed-up state by seeding the HD and CC pipelines over 55
    /// bars, then processes bar 55 (the first valid output).
    ///
    /// **Three phases:**
    /// 1. Bars 0–5:  CC seeding (second-difference formula) + `hd.calc()` (safe).
    /// 2. Bars 6–21: `hd.calc()` (safe) + `cc.calc_unchecked()` + Fisher tracking.
    /// 3. Bars 22–54: `hd.calc_unchecked()` + `cc.calc_unchecked()` + Fisher tracking.
    ///
    /// Writes the first output values to the respective slices at index 0.
    /// Pass empty slices (`&mut []`) for any optional output that is not needed.
    pub fn init_state(
        real: &[f64],
        alpha: f64, // 0.0 = adaptive; (0,1) = fixed
        fisher_line: &mut [f64],
        signal_line: &mut [f64],
        trendmode_line: &mut [f64],
        cycle_line: &mut [f64],
        peak_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new();
        let fixed_mults = if alpha > 0.0 {
            Some(cybercycle::multiplier(alpha))
        } else {
            None
        };

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
            let mults = match fixed_mults {
                Some(m) => m,
                None => cybercycle::multiplier(cybercycle::adaptive_alpha(state.hd.smooth_period)),
            };
            let cycle = unsafe { state.cc.calc_unchecked(real[i], mults) };
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
            unsafe { state.hd.calc_unchecked(real[i]) };
            let mults = match fixed_mults {
                Some(m) => m,
                None => cybercycle::multiplier(cybercycle::adaptive_alpha(state.hd.smooth_period)),
            };
            let cycle = unsafe { state.cc.calc_unchecked(real[i], mults) };
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
            unsafe { state.calc_unchecked_adaptive(real[55]) }
        } else {
            unsafe { state.calc_unchecked(real[55], fixed_mults.unwrap()) }
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

        state
    }

    /// Unsafe one-bar update — skips all ring-buffer fullness guards.
    ///
    /// After the call:
    /// - `state.hd.smooth_period` = DC period (current bar)
    /// - `state.cc.cycle_prev`    = Cycle (current bar)
    /// - `state.pk`               = peak amplitude (current bar)
    /// - `state.val1`             = smoothed normalised value (current bar)
    /// - `state.fish`             = Fisher value (current bar)
    ///
    /// Returns `(fisher, signal)` where `signal` is the Fisher value from the
    /// previous bar.
    ///
    /// # Safety
    ///
    /// All HD and CC ring buffers must be full on entry.
    /// Guaranteed after [`init_state`](Self::init_state).
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        price: f64,
        multipliers: (f64, f64, f64),
    ) -> (f64, f64) {
        self.hd.calc_unchecked(price);
        let cycle = self.cc.calc_unchecked(price, multipliers);
        self.pk = (self.pk * 0.991).max(cycle.abs());
        let value = if self.pk > 0.0 { cycle / self.pk } else { 0.0 };
        self.val1 = (0.65 * self.val1 + 0.35 * value).clamp(-0.999, 0.999);
        let ln_arg = (1.0 + self.val1) / (1.0 - self.val1);
        let fish = 0.5 * ln_arg.ln();
        let signal = self.fish;
        self.fish = fish;
        (fish, signal)
    }

    /// Unsafe one-bar update using **adaptive alpha** derived from the HD's `smooth_period`.
    ///
    /// Returns `(fisher, signal)` where signal is Fisher[1] (previous bar).
    ///
    /// # Safety
    /// All HD and CC ring buffers must be full on entry.
    /// Guaranteed after [`init_state`](Self::init_state).
    #[inline(always)]
    pub unsafe fn calc_unchecked_adaptive(&mut self, price: f64) -> (f64, f64) {
        self.hd.calc_unchecked(price);
        let alpha = cybercycle::adaptive_alpha(self.hd.smooth_period);
        let multipliers = cybercycle::multiplier(alpha);
        let cycle = self.cc.calc_unchecked(price, multipliers);
        self.pk = (self.pk * 0.991).max(cycle.abs());
        let value = if self.pk > 0.0 { cycle / self.pk } else { 0.0 };
        self.val1 = (0.65 * self.val1 + 0.35 * value).clamp(-0.999, 0.999);
        let ln_arg = (1.0 + self.val1) / (1.0 - self.val1);
        let fish = 0.5 * ln_arg.ln();
        let signal = self.fish;
        self.fish = fish;
        (fish, signal)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistent state for streaming / multi-batch use.
///
/// Stores the precomputed filter coefficients alongside the filter state,
/// mirroring the [`cybercycle::IndicatorState`] pattern.
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub(crate) alpha: f64,
    pub(crate) multipliers: (f64, f64, f64),
    pub(crate) state: State,
}

impl IndicatorState {
    pub fn new(state: State, alpha: f64) -> Self {
        let multipliers = if alpha > 0.0 {
            cybercycle::multiplier(alpha)
        } else {
            (0.0, 0.0, 0.0)
        };
        Self {
            alpha,
            multipliers,
            state,
        }
    }
}

impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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
            &mut self.state,
            self.alpha,
            self.multipliers,
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

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the minimum number of input bars required for any output.
///
/// Bars 0–54 are absorbed by the HD + CC + Fisher warmup; bar 55 is the first
/// valid output.
pub fn min_data(_options: &[f64]) -> usize {
    56
}

/// `min_data` independent of decimal accuracy (IIR/HD with fixed structure).
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Number of output bars for a given input length.
pub fn output_length(data_len: usize, _options: &[f64]) -> usize {
    data_len.saturating_sub(55)
}

/// Validates `alpha`.
///
/// * `0.0` — adaptive (derived from `SmoothPeriod` each bar via the embedded HD).
/// * `(0.0, 1.0)` — fixed user-supplied alpha. Ehlers' default is `0.07`.
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 0.0 || options[0] >= 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Calculates the Ehlers CyberCycle Fisher over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close (or HLC/3) price series
///
/// # Options
///
/// * `options[0]` — `alpha` ∈ (0, 1).  Ehlers' default is `0.07`.
///
/// # Outputs
///
/// * `outputs[0]` — `fisher`:    Fisher Transform value
/// * `outputs[1]` — `signal`:    trigger line = Fisher[1]
/// * `outputs[2]` — `trendmode`: `1.0` = Trend, `0.0` = Cycle (optional)
/// * `outputs[3]` — `cycle`:     raw CyberCycle oscillator (optional)
/// * `outputs[4]` — `peak`:      decaying amplitude peak (optional)
///
/// # Returns
///
/// `Ok((outputs, state))` where `state` can be used for streaming via
/// [`IndicatorState::batch_indicator`].  Returns `Err` if inputs are too short
/// or `alpha` is outside `(0, 1)`.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let alpha = options[0];
    let multipliers = if alpha > 0.0 {
        cybercycle::multiplier(alpha)
    } else {
        (0.0, 0.0, 0.0)
    };
    let real = inputs[0];
    let n = real.len();
    let capacity = output_length(n, options);

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
        let o = crate::slice_outputs_start!(capacity - 1, trendmode_line, cycle_line, peak_line);
        (
            &mut trendmode_line[o.0..],
            &mut cycle_line[o.1..],
            &mut peak_line[o.2..],
        )
    };

    // Process bars 56..n (output indices 1..capacity).
    run_ccfisher(
        &real[min_data(options)..],
        &mut state,
        alpha,
        multipliers,
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
        IndicatorState::new(state, alpha),
    ))
}

/// Shared hot loop used by both `indicator` and `batch_indicator`.
///
/// All HD and CC ring buffers must be full on entry (guaranteed after
/// `init_state`).  Writes `fisher` and `signal` for every bar, and optionally
/// `trendmode`, `cycle`, and `peak`.
fn run_ccfisher(
    real: &[f64],
    state: &mut State,
    alpha: f64,
    multipliers: (f64, f64, f64),
    fisher_line: &mut [f64],
    signal_line: &mut [f64],
    trendmode_line: &mut [f64],
    cycle_line: &mut [f64],
    peak_line: &mut [f64],
) {
    let (has_optional, want_trendmode, want_cycle, want_peak) =
        crate::calc_want_flags!(trendmode_line, cycle_line, peak_line);
    if alpha == 0.0 {
        for i in 0..real.len() {
            let (fisher, signal) = unsafe { state.calc_unchecked_adaptive(*real.get_unchecked(i)) };
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
            let (fisher, signal) =
                unsafe { state.calc_unchecked(*real.get_unchecked(i), multipliers) };
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
