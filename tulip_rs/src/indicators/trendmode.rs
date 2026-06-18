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

use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{cybercycle, homodynediscriminator};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1; // [alpha]

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

/// Metadata for the Ehlers TrendMode indicator.
pub const INFO: Info = Info {
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
pub struct State {
    /// Embedded Homodyne Discriminator — provides `SmoothPeriod` (DC) per bar.
    pub hd: homodynediscriminator::State,
    /// Embedded CyberCycle oscillator — produces `Cycle` per bar.
    pub cc: cybercycle::State,
    /// Running peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
    pub pk: f64,
}

impl State {
    /// Creates a zeroed state ready for the first bar.
    pub fn new() -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            cc: cybercycle::State::new(),
            pk: 0.0,
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
        multipliers: (f64, f64, f64),
        trendmode_line: &mut [f64],
        cycle_line: &mut [f64],
        peak_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new();

        // ── Phase 1: bars 0–5 — CC seeding + HD warmup ───────────────────────
        // Mirrors cybercycle::State::seed_warmup but also feeds the HD pipeline.
        // Bars 0–1: price_buf.len() < 3 → seeding formula cannot run; cycle stays 0.
        // Bar 2:    first seed value (len = 3).
        // Bars 3–5: smooth also becomes available (len = 4).
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
        // After loop: CC ring buffers full; HD still warming up.

        // ── Phase 2: bars 6–21 — HD safe + CC unchecked + peak tracking ──────
        // HD reaches fully-seeded at bar 21; smooth_period = 0 before that bar,
        // so dc_period clamps to 6 for bars 6–20 and rises from bar 21 onwards.
        for i in 6..22 {
            state.hd.calc(real[i]);
            let cycle = unsafe { state.cc.calc_unchecked(real[i], multipliers) };
            state.pk = (state.pk * 0.991).max(cycle.abs());
        }

        // ── Phase 3: bars 22–54 — both unchecked + peak tracking ─────────────
        for i in 22..55 {
            unsafe { state.hd.calc_unchecked(real[i]) };
            let cycle = unsafe { state.cc.calc_unchecked(real[i], multipliers) };
            state.pk = (state.pk * 0.991).max(cycle.abs());
        }

        // ── Bar 55: first valid output ────────────────────────────────────────
        let trendmode = unsafe { state.calc_unchecked(real[55], multipliers) };
        trendmode_line[0] = trendmode;
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
    ///
    /// Returns `1.0` (Trend Mode) or `0.0` (Cycle Mode).
    ///
    /// # Safety
    ///
    /// All HD and CC ring buffers must be full on entry.
    /// Guaranteed after [`init_state`](Self::init_state).
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, price: f64, multipliers: (f64, f64, f64)) -> f64 {
        self.hd.calc_unchecked(price);
        let cycle = self.cc.calc_unchecked(price, multipliers);
        self.pk = (self.pk * 0.991).max(cycle.abs());
        if self.pk > 0.0 && cycle.abs() < 0.2 * self.pk {
            1.0
        } else {
            0.0
        }
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
/// mirroring the `cybercycle::IndicatorState` pattern.
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub(crate) multipliers: (f64, f64, f64),
    pub(crate) state: State,
}

impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64, f64)) -> Self {
        Self { multipliers, state }
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
        let want_cycle = optional_outputs
            .and_then(|f| f.first().copied())
            .unwrap_or(false);
        let want_peak = optional_outputs
            .and_then(|f| f.get(1).copied())
            .unwrap_or(false);

        let mut trendmode_line = crate::uninit_vec!(f64, n);
        let mut cycle_line: Vec<f64> = if want_cycle {
            crate::uninit_vec!(f64, n)
        } else {
            Vec::new()
        };
        let mut peak_line: Vec<f64> = if want_peak {
            crate::uninit_vec!(f64, n)
        } else {
            Vec::new()
        };

        run_trendmode(
            real,
            &mut self.state,
            self.multipliers,
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

/// Returns the minimum number of input bars required for any output.
///
/// Bars 0–54 are absorbed by the HD + CC + peak warmup; bar 55 is the first
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

/// Validates that `alpha` is strictly in `(0.0, 1.0)`.
///
/// Delegates to [`cybercycle::validate_options`] which uses the same range.
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    cybercycle::validate_options(options)
}

/// Calculates the Ehlers TrendMode over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — close (or HLC/3) price series
///
/// # Options
///
/// * `options[0]` — `alpha` ∈ (0, 1). Ehlers' default is `0.07`.
///
/// # Outputs
///
/// * `outputs[0]` — `trendmode`: `1.0` = Trend Mode, `0.0` = Cycle Mode
/// * `outputs[1]` — `cycle`:    CyberCycle oscillator (optional; empty unless requested)
/// * `outputs[2]` — `peak`:     decaying amplitude peak (optional; empty unless requested)
///
/// # Returns
///
/// `Ok((outputs, state))` where `state` can be used for streaming via
/// [`IndicatorState::batch_indicator`]. Returns `Err` if inputs are too short
/// or `alpha` is outside `(0, 1)`.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let alpha = options[0];
    let multipliers = cybercycle::multiplier(alpha);
    let real = inputs[0];
    let n = real.len();
    let capacity = output_length(n, options);

    let want_cycle = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);
    let want_peak = optional_outputs
        .and_then(|f| f.get(1).copied())
        .unwrap_or(false);

    let mut trendmode_line = crate::uninit_vec!(f64, capacity);
    let mut cycle_line: Vec<f64> = if want_cycle {
        crate::uninit_vec!(f64, capacity)
    } else {
        Vec::new()
    };
    let mut peak_line: Vec<f64> = if want_peak {
        crate::uninit_vec!(f64, capacity)
    } else {
        Vec::new()
    };

    // init_state seeds bars 0–54 and processes bar 55 (output index 0).
    let mut state = State::init_state(
        real,
        multipliers,
        &mut trendmode_line,
        &mut cycle_line,
        &mut peak_line,
    );

    let cycle_tail = if want_cycle {
        &mut cycle_line[1..]
    } else {
        &mut cycle_line[..]
    };
    let peak_tail = if want_peak {
        &mut peak_line[1..]
    } else {
        &mut peak_line[..]
    };

    // Process bars 56..n (output indices 1..capacity).
    run_trendmode(
        &real[min_data(options)..],
        &mut state,
        multipliers,
        &mut trendmode_line[1..],
        cycle_tail,
        peak_tail,
    );

    Ok((
        vec![trendmode_line, cycle_line, peak_line],
        IndicatorState::new(state, multipliers),
    ))
}

/// Shared hot loop used by both `indicator` and `batch_indicator`.
///
/// All HD and CC ring buffers must be full on entry (guaranteed after
/// `init_state`). Writes `trendmode` for every bar, and optionally `cycle` and
/// `peak`.
pub fn run_trendmode(
    real: &[f64],
    state: &mut State,
    multipliers: (f64, f64, f64),
    trendmode_line: &mut [f64],
    cycle_line: &mut [f64],
    peak_line: &mut [f64],
) {
    let (has_optional, want_cycle, want_peak) = crate::calc_want_flags!(cycle_line, peak_line);
    for i in 0..real.len() {
        let trendmode = unsafe { state.calc_unchecked(*real.get_unchecked(i), multipliers) };
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
