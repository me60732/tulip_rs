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
//! every valid α value. The local function checks `α ∈ (0.0, 1.0)` strictly.
//!
//! ## Adaptive alpha (`α = 0.0`)
//!
//! Adaptive alpha is **not** supported by the standalone `cybercycle::indicator`.
//! It requires a Homodyne Discriminator (HD) to derive `SmoothPeriod` each bar,
//! which is not part of this indicator. Passing `α = 0.0` will return
//! [`IndicatorError::InvalidOptions`].
//!
//! Adaptive mode is available in [`trendmode`](super::trendmode) and
//! [`ccfisher`](super::ccfisher), both of which embed an HD alongside the
//! CyberCycle and compute `α = 2 / (SmoothPeriod.max(3) + 1)` every bar.

use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::ring_buffer::fixed_single_buffer::FixedRingBuffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1; // [alpha]

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

/// `IndicatorState` is the complete self-contained state — coefficients live inside
/// `State` alongside the filter history, matching the `ema::IndicatorState` pattern.
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
        let mut cycle_line = crate::uninit_vec!(f64, n);
        let mut trigger_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            trigger_line: n
        );

        run_cycle(real, self, &mut cycle_line, &mut trigger_line);

        Ok(vec![cycle_line, trigger_line])
    }
}

/// Per-bar filter state for the Ehlers CyberCycle.
///
/// Stores the precomputed IIR coefficients (`coef`, `d1`, `d2`) alongside the
/// filter history, so `calc` / `calc_unchecked` need no external parameters.
///
/// **Warmup:** after [`State::init_state`] completes, all ring buffers are full and
/// the IIR feedback is seeded. The hot path (`calc_unchecked`) operates unconditionally.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// 4-bar price ring buffer: `[0]`=Price, `[1]`=Price[1], `[2]`=Price[2], `[3]`=Price[3].
    pub price_buf: FixedRingBuffer<f64, 4, S>,

    /// 3-bar smooth ring buffer: `[0]`=Smooth, `[1]`=Smooth[1], `[2]`=Smooth[2].
    pub smooth_buf: FixedRingBuffer<f64, 3, S>,

    /// Cycle[1] — one-bar-ago cycle value (IIR feedback state d₁).
    pub cycle_prev: f64,

    /// Cycle[2] — two-bar-ago cycle value (IIR feedback state d₂).
    pub cycle_prev2: f64,
    pub coef: f64,
    pub d1: f64,
    pub d2: f64,
}

impl Default for State<Cold> {
    fn default() -> Self {
        Self {
            price_buf: FixedRingBuffer::new(),
            smooth_buf: FixedRingBuffer::new(),
            cycle_prev: 0.0,
            cycle_prev2: 0.0,
            coef: 0.0,
            d1: 0.0,
            d2: 0.0,
        }
    }
}

impl State<Cold> {
    /// Creates a zeroed filter state and precomputes coefficients from `alpha`.
    pub fn new(alpha: f64) -> State<Cold> {
        let (coef, d1, d2) = multiplier(alpha);
        Self {
            price_buf: FixedRingBuffer::new(),
            smooth_buf: FixedRingBuffer::new(),
            cycle_prev: 0.0,
            cycle_prev2: 0.0,
            coef,
            d1,
            d2,
        }
    }
    pub fn into_full(self) -> State<Warm>{
        State {
            price_buf: self.price_buf.into_full(),
            smooth_buf: self.smooth_buf.into_full(),
            cycle_prev: self.cycle_prev,
            cycle_prev2: self.cycle_prev2,
            coef: self.coef,
            d1: self.d1,
            d2: self.d2,
        }
    }
    /// Seeds the IIR through bars 0–5 **without** processing bar 6.
    ///
    /// Used by the `by_options` SIMD path where the driver writes bar 6's output
    /// directly. The returned state has both ring buffers full and
    /// `cycle_prev`/`cycle_prev2` seeded from the second-difference formula.
    pub fn seed_warmup(real: &[f64], alpha: f64) -> State<Warm> {
        let mut state = Self::new(alpha);
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
        State {
            price_buf: state.price_buf.into_full(),
            smooth_buf: state.smooth_buf.into_full(),
            cycle_prev: state.cycle_prev,
            cycle_prev2: state.cycle_prev2,
            coef: state.coef,
            d1: state.d1,
            d2: state.d2,
        }
    }

    /// Seeds the IIR for bars 0–5, then processes bar 6 (first valid output).
    ///
    /// Writes `cycle_line[0]` and (if non-empty) `trigger_line[0]`.
    /// After the call, all ring buffers are full and `calc_unchecked` is safe.
    pub fn init_state(
        real: &[f64],
        alpha: f64,
        cycle_line: &mut [f64],
        trigger_line: &mut [f64],
    ) -> State<Warm> {
        let mut state = Self::new(alpha);

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
        let cycle = state.calc(real[6]);
        cycle_line[0] = cycle;
        // After calc_unchecked: cycle_prev = Cycle[6], cycle_prev2 = Cycle[5].
        // Trigger[0] = Cycle[5] = the last seeded cycle before bar 6.
        if !trigger_line.is_empty() {
            trigger_line[0] = state.cycle_prev2;
        }

        State {
            price_buf: state.price_buf.into_full(),
            smooth_buf: state.smooth_buf.into_full(),
            cycle_prev: state.cycle_prev,
            cycle_prev2: state.cycle_prev2,
            coef: state.coef,
            d1: state.d1,
            d2: state.d2,
        }
    }

}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        // ── Stage 1: 6-tap weighted smooth ──────────────────────────────────
        // Smooth = (P + 2·P[1] + 2·P[2] + P[3]) / 6
        self.price_buf.push(price);
        let ab = 2.0_f64.mul_add(self.price_buf[1], self.price_buf[0]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * (1.0 / 6.0);

        // ── Stage 2: 2-pole high-pass IIR ───────────────────────────────────
        // Cycle = coef·(S − 2·S[1] + S[2]) + d1·C[1] − d2·C[2]
        self.smooth_buf.push(smooth);
        let smooth_diff = (-2.0_f64).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = self.coef.mul_add(
            smooth_diff,
            self.d1
                .mul_add(self.cycle_prev, -self.d2 * self.cycle_prev2),
        );

        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    /// Safe single-bar update — handles ring-buffer warmup guards internally.
    /// Returns `0.0` during the first 5 bars while the buffers are filling.
    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
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
        let smooth_diff = (-2.0_f64).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = self.coef.mul_add(
            smooth_diff,
            self.d1
                .mul_add(self.cycle_prev, -self.d2 * self.cycle_prev2),
        );
        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }
}

/// Validates that `alpha` is strictly in `(0.0, 1.0)`.
///
/// `alpha = 0.0` is rejected — adaptive mode is not available in the standalone
/// CyberCycle indicator (no embedded HD). Use [`trendmode`](super::trendmode) or
/// [`ccfisher`](super::ccfisher) for adaptive alpha.
///
/// **Do not** use `crate::common::validate_options` here — it rejects any
/// option `< 1.0` and would flag all valid α values.
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
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

/// Computes adaptive alpha from the Homodyne Discriminator's `smooth_period`.
///
/// `alpha = 2 / (smooth_period.max(3) + 1)`, keeping alpha in `(0, 0.5]`.
/// Clamping to `max(3)` prevents alpha from exceeding 0.5 when `smooth_period`
/// is near zero during HD warmup (first ~22 bars of the indicator's 55-bar warmup).
///
/// This is the Ehlers α-from-period conversion: the dominant cycle period
/// acts as the EMA's equivalent period, and alpha is the corresponding coefficient.
#[inline(always)]
pub fn adaptive_alpha(smooth_period: f64) -> f64 {
    2.0 / (smooth_period.max(3.0) + 1.0)
}

/// Shared hot loop used by both `indicator` and `batch_indicator`.
///
/// After each bar: `state.cycle_prev2` = Cycle[1] = trigger for that bar.
fn run_cycle(real: &[f64], state: &mut State<Warm>, cycle_line: &mut [f64], trigger_line: &mut [f64]) {
    let want_trigger = !trigger_line.is_empty();
    for i in 0..real.len() {
        unsafe {
            *cycle_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
        }
        crate::store_optional_outputs!(i,
            want_trigger, trigger_line => state.cycle_prev2
        );
    }
}

pub struct Cybercycle;
impl Indicator<INPUTS, OPTIONS> for Cybercycle {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "cybercycle",
        indicator_type: IndicatorType::Cycle,
        full_name: "Ehlers CyberCycle",
        inputs: &["real"],
        options: &["alpha"],
        outputs: &["cybercycle"],
        optional_outputs: &["trigger"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "cybercycle",
            label: "Ehlers Cyber Cycle",
            display_type: DisplayType::Indicator,
            outputs: &["cybercycle", "trigger"],
        }],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        7
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;

        let real = inputs[0];
        let n = real.len();
        let capacity = Self::output_length(n, options);
        let mut cycle_line = crate::uninit_vec!(f64, capacity);
        let mut trigger_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            trigger_line: capacity
        );

        // init_state seeds bars 0–5 and processes bar 6 (output index 0).
        let mut state = State::init_state(real, options[0], &mut cycle_line, &mut trigger_line);

        // Process bars 7..n (output indices 1..capacity).
        let trigger_start = crate::slice_outputs_start!(capacity - 1, trigger_line);
        run_cycle(
            &real[Self::min_data(options)..],
            &mut state,
            &mut cycle_line[1..],
            &mut trigger_line[trigger_start..],
        );

        Ok((vec![cycle_line, trigger_line], state))
    }
}
