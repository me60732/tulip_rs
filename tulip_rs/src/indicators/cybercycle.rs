//! # Ehlers CyberCycle
//!
//! **Source:** John Ehlers, *Cybernetic Analysis for Stocks and Futures* (2004), Chapter 4.
//!
//! A two-pole high-pass IIR that removes the low-frequency trend (DC and sub-cycle
//! drift) from price, leaving only the dominant short-cycle oscillation. It provides
//! cycle-mode entry/exit signals via a crossover of `Cycle` and `Trigger`.
//!
//! ## Formula
//!
//! ```text
//! c  = 1 − α/2,   b  = 1 − α         (α = options[0], default 0.07)
//!
//! Smooth = (Price + 2·Price[1] + 2·Price[2] + Price[3]) / 6
//!
//! Seeding (bars 0–5, absorbed by init_state):
//!   Cycle = (Price − 2·Price[1] + Price[2]) / 4
//!
//! Steady state (bar ≥ 6):
//!   Cycle = c²·(Smooth − 2·Smooth[1] + Smooth[2])
//!         + 2b·Cycle[1] − b²·Cycle[2]
//!
//! Trigger = Cycle[1]   (optional 1-bar lag — leads by ~1 bar)
//! ```
//!
//! ## Note on `validate_options`
//!
//! This indicator uses a **local** `validate_options` function. The common
//! `crate::common::validate_options` rejects any option `< 1.0`, which would flag
//! every valid α value. The local function checks `0.0 < α < 1.0` instead.

use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::ring_buffer::fixed_single_buffer::FixedRingBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1; // [alpha]

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::cybercycle_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::cybercycle_simd::indicator_by_options;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::cybercycle_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes one asset with `N` different alpha values in parallel.
    pub use crate::indicators::simd_indicators::cybercycle_simd::indicator_by_options as indicator;
}

/// Metadata for the Ehlers CyberCycle indicator.
pub const INFO: Info = Info {
    name: "cybercycle",
    indicator_type: IndicatorType::Cycle,
    full_name: "Ehlers CyberCycle",
    inputs: &["real"],
    options: &["alpha"],
    outputs: &["cybercycle"],
    optional_outputs: &["trigger"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "cybercycle",
            label: "Ehlers CyberCycle",
            display_type: DisplayType::Indicator,
            outputs: &["cybercycle"],
        },
        DisplayGroup {
            offset: None,
            id: "cybercycle_trigger",
            label: "CyberCycle Trigger",
            display_type: DisplayType::Indicator,
            outputs: &["trigger"],
        },
    ],
};

/// Persistent state for streaming / multi-batch use.
///
/// Stores the precomputed filter coefficients alongside the filter state,
/// exactly mirroring the `supersmoother::IndicatorState` pattern.
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
        let want_trigger = optional_outputs
            .and_then(|f| f.first().copied())
            .unwrap_or(false);

        let mut cycle_line = crate::uninit_vec!(f64, n);
        let mut trigger_line: Vec<f64> = if want_trigger {
            crate::uninit_vec!(f64, n)
        } else {
            Vec::new()
        };

        run_cycle(
            real,
            &mut self.state,
            self.multipliers,
            &mut cycle_line,
            if want_trigger {
                &mut trigger_line
            } else {
                &mut []
            },
        );

        Ok(vec![cycle_line, trigger_line])
    }
}

/// Per-bar filter state for the Ehlers CyberCycle.
///
/// **Warmup:** after [`init_state`](State::init_state) completes, all ring
/// buffers are full and the IIR feedback is seeded. The hot path
/// (`calc_unchecked`) operates unconditionally.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// 4-bar price ring buffer: `[0]`=Price, `[1]`=Price[1], `[2]`=Price[2], `[3]`=Price[3].
    pub price_buf: FixedRingBuffer<f64, 4>,

    /// 3-bar smooth ring buffer: `[0]`=Smooth, `[1]`=Smooth[1], `[2]`=Smooth[2].
    pub smooth_buf: FixedRingBuffer<f64, 3>,

    /// Cycle[1] — one-bar-ago cycle value (IIR feedback state d₁).
    pub cycle_prev: f64,

    /// Cycle[2] — two-bar-ago cycle value (IIR feedback state d₂).
    pub cycle_prev2: f64,
}

impl State {
    /// Creates a zeroed state ready for the first bar.
    pub fn new() -> Self {
        Self {
            price_buf: FixedRingBuffer::new(),
            smooth_buf: FixedRingBuffer::new(),
            cycle_prev: 0.0,
            cycle_prev2: 0.0,
        }
    }

    /// Seeds the IIR through bars 0–5 **without** processing bar 6.
    ///
    /// Used by the `by_options` SIMD path where the driver writes bar 6's output
    /// directly. The returned state has both ring buffers full and
    /// `cycle_prev`/`cycle_prev2` seeded from the second-difference formula.
    pub fn seed_warmup(real: &[f64]) -> Self {
        let mut state = Self::new();
        for i in 0..6 {
            state.price_buf.push(real[i]);
            if state.price_buf.len() >= 4 {
                let ab = 2.0_f64.mul_add(state.price_buf[1], state.price_buf[0]);
                let cd = 2.0_f64.mul_add(state.price_buf[2], state.price_buf[3]);
                state.smooth_buf.push((ab + cd) * (1.0 / 6.0));
            }
            if state.price_buf.len() >= 3 {
                let seed =
                    (state.price_buf[0] - 2.0 * state.price_buf[1] + state.price_buf[2]) / 4.0;
                state.cycle_prev2 = state.cycle_prev;
                state.cycle_prev = seed;
            }
        }
        state
    }

    /// Seeds the IIR for bars 0–5, then processes bar 6 (first valid output).
    ///
    /// Writes `cycle_line[0]` and (if non-empty) `trigger_line[0]`.
    /// After the call, all ring buffers are full and `calc_unchecked` is safe.
    pub fn init_state(
        real: &[f64],
        multipliers: (f64, f64, f64),
        cycle_line: &mut [f64],
        trigger_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new();

        // ── Seeding: bars 0–5 ────────────────────────────────────────────────
        // Bars 0–1: price_buf.len() < 3 → seeding formula cannot run; cycle stays 0.
        // Bar 2:    first seeding value.
        // Bars 3–5: Smooth also becomes available (price_buf.len() >= 4).
        for i in 0..6 {
            state.price_buf.push(real[i]);

            if state.price_buf.len() >= 4 {
                let ab = 2.0_f64.mul_add(state.price_buf[1], state.price_buf[0]);
                let cd = 2.0_f64.mul_add(state.price_buf[2], state.price_buf[3]);
                state.smooth_buf.push((ab + cd) * (1.0 / 6.0));
            }

            if state.price_buf.len() >= 3 {
                let seed =
                    (state.price_buf[0] - 2.0 * state.price_buf[1] + state.price_buf[2]) / 4.0;
                state.cycle_prev2 = state.cycle_prev;
                state.cycle_prev = seed;
            }
        }
        // After loop: price_buf = [P5,P4,P3,P2] (full)
        //             smooth_buf = [S5,S4,S3]   (full — first three smooths)
        //             cycle_prev  = Cycle[5]
        //             cycle_prev2 = Cycle[4]

        // ── Bar 6: first valid output ─────────────────────────────────────────
        let cycle = unsafe { state.calc_unchecked(real[6], multipliers) };
        cycle_line[0] = cycle;
        // After calc_unchecked: cycle_prev = Cycle[6], cycle_prev2 = Cycle[5].
        // Trigger[0] = Cycle[5] = the last seeded cycle before bar 6.
        if !trigger_line.is_empty() {
            trigger_line[0] = state.cycle_prev2;
        }

        state
    }

    /// Safe one-bar update. Returns `0.0` while ring buffers are still filling.
    ///
    /// Prefer `calc_unchecked` in hot loops after [`init_state`](Self::init_state).
    #[inline(always)]
    pub fn calc(&mut self, price: f64, multipliers: (f64, f64, f64)) -> f64 {
        self.price_buf.push(price);
        if self.price_buf.len() < 4 {
            return 0.0;
        }
        let ab = 2.0_f64.mul_add(self.price_buf[1], self.price_buf[0]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * (1.0 / 6.0);
        self.smooth_buf.push(smooth);
        if self.smooth_buf.len() < 3 {
            return 0.0;
        }
        let (coeff, d1, d2) = multipliers;
        let smooth_diff = (-2.0_f64).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = coeff.mul_add(
            smooth_diff,
            d1.mul_add(self.cycle_prev, -d2 * self.cycle_prev2),
        );
        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }

    /// Unsafe one-bar update — skips ring-buffer fullness guards.
    ///
    /// After the call:
    /// - `state.cycle_prev`  = Cycle (current bar)
    /// - `state.cycle_prev2` = Cycle[1] (previous bar)
    /// - Trigger = `state.cycle_prev2`
    ///
    /// # Safety
    ///
    /// Both `price_buf` and `smooth_buf` must be full on entry.
    /// Guaranteed after [`init_state`](Self::init_state).
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, price: f64, multipliers: (f64, f64, f64)) -> f64 {
        // ── Stage 1: 6-tap weighted smooth ──────────────────────────────────
        // Smooth = (P + 2·P[1] + 2·P[2] + P[3]) / 6
        // Decomposed into 2 FMAs (serial depth 2):
        //   ab = 2·P[1] + P   = P + 2·P[1]
        //   cd = 2·P[2] + P[3]
        self.price_buf.push_unchecked(price);
        let ab = 2.0_f64.mul_add(self.price_buf[1], self.price_buf[0]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * (1.0 / 6.0);

        // ── Stage 2: 2-pole high-pass IIR ───────────────────────────────────
        // Cycle = coeff·(S − 2·S[1] + S[2]) + d1·C[1] − d2·C[2]
        // Three FMAs (serial depth 2):
        //   smooth_diff = −2·S[1] + S  (FMA) + S[2]
        //   inner       = d1·C[1] − d2·C[2]
        //   cycle       = coeff·smooth_diff + inner
        self.smooth_buf.push_unchecked(smooth);
        let (coeff, d1, d2) = multipliers;
        let smooth_diff = (-2.0_f64).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = coeff.mul_add(
            smooth_diff,
            d1.mul_add(self.cycle_prev, -d2 * self.cycle_prev2),
        );

        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the minimum number of input bars required for any output.
///
/// Bars 0–5 are absorbed by seeding; bar 6 is the first valid output.
pub fn min_data(_options: &[f64]) -> usize {
    7
}

/// `min_data` independent of decimal accuracy (IIR with fixed coefficients).
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Number of output bars for a given input length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Validates that `alpha` is strictly in `(0.0, 1.0)`.
///
/// **Do not** use `crate::common::validate_options` here — it rejects any
/// option `< 1.0` and would flag all valid α values.
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] <= 0.0 || options[0] >= 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Precomputes the three stable IIR multipliers from `alpha`.
///
/// Returns `(coeff, d1, d2)` where:
/// - `coeff` = `(1 − α/2)²` — feedforward gain
/// - `d1`    = `2·(1 − α)` — first feedback coefficient
/// - `d2`    = `(1 − α)²`  — second feedback coefficient
pub fn multiplier(alpha: f64) -> (f64, f64, f64) {
    let c = 1.0 - 0.5 * alpha;
    let b = 1.0 - alpha;
    (c * c, 2.0 * b, b * b)
}

/// Calculates the Ehlers CyberCycle over the full input dataset.
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
/// * `outputs[0]` — `cybercycle` oscillator
/// * `outputs[1]` — `trigger` = Cycle[1] (optional; empty unless requested)
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
    let mults = multiplier(alpha);
    let real = inputs[0];
    let n = real.len();
    let capacity = output_length(n, options);
    let want_trigger = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);

    let mut cycle_line = crate::uninit_vec!(f64, capacity);
    let mut trigger_line: Vec<f64> = if want_trigger {
        crate::uninit_vec!(f64, capacity)
    } else {
        Vec::new()
    };

    // init_state seeds bars 0–5 and processes bar 6 (output index 0).
    let mut state = State::init_state(real, mults, &mut cycle_line, &mut trigger_line);

    // Process bars 7..n (output indices 1..capacity).
    run_cycle(
        &real[min_data(options)..],
        &mut state,
        mults,
        &mut cycle_line[1..],
        if want_trigger {
            &mut trigger_line[1..]
        } else {
            &mut []
        },
    );

    Ok((
        vec![cycle_line, trigger_line],
        IndicatorState::new(state, mults),
    ))
}

/// Shared hot loop used by both `indicator` and `batch_indicator`.
///
/// After each bar: `state.cycle_prev2` = Cycle[1] = trigger for that bar.
fn run_cycle(
    real: &[f64],
    state: &mut State,
    multipliers: (f64, f64, f64),
    cycle_line: &mut [f64],
    trigger_line: &mut [f64],
) {
    let want_trigger = !trigger_line.is_empty();
    for i in 0..real.len() {
        unsafe {
            *cycle_line.get_unchecked_mut(i) =
                state.calc_unchecked(*real.get_unchecked(i), multipliers);
            if want_trigger {
                *trigger_line.get_unchecked_mut(i) = state.cycle_prev2;
            }
        }
    }
}
