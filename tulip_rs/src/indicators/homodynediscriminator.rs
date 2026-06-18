//! # Ehlers Homodyne Discriminator
//!
//! **Source:** John Ehlers, *Rocket Science for Traders* (2001), Chapter 7.
//! Also published in *Technical Analysis of Stocks & Commodities*, December 2000.
//!
//! Measures the dominant cycle period in the market bar-by-bar using the
//! Homodyne Discriminator technique (an analogue signal-processing method adapted
//! for discrete price data). It is a prerequisite for the [`mama`] indicator and
//! for any other indicator that needs an adaptive measure of market cycle length.
//!
//! ## Pipeline (4 stages)
//!
//! ```text
//! Stage 0 — 4-bar Hann smooth
//!     Smooth = (4·P + 3·P[1] + 2·P[2] + P[3]) / 10
//!
//! Stage 1 — Detrender (7-tap adaptive Hilbert kernel on Smooth)
//!     gain  = 0.075·Period[1] + 0.54
//!     Q_det = (0.0962·S[0] + 0.5769·S[2] − 0.5769·S[4] − 0.0962·S[6]) · gain
//!     I_det = S[3]
//!
//! Stage 2 — I1 / Q1 (same adaptive kernel on Detrender)
//!     I1 = Detrender[3],   Q1 = kernel(Detrender) · gain
//!
//! Stage 3 — jI / jQ (kernel on I1 and Q1 simultaneously)
//!     jI = kernel(I1) · gain,   jQ = kernel(Q1) · gain
//!
//! Homodyne discriminator:
//!     I2 = I1 − jQ,   Q2 = Q1 + jI   (phase rotation)
//!     I2 = 0.2·I2 + 0.8·I2[1]        (IIR smooth)
//!     Q2 = 0.2·Q2 + 0.8·Q2[1]
//!     Re = I2·I2[1] + Q2·Q2[1]
//!     Im = I2·Q2[1] − Q2·I2[1]
//!     Re = 0.2·Re + 0.8·Re[1]
//!     Im = 0.2·Im + 0.8·Im[1]
//!
//! Period measurement & smoothing:
//!     Period = 2π / atan(Im / Re)     (clamped to [6, 50] bars, ±50% change limit)
//!     SmoothPeriod = 0.33·Period + 0.67·SmoothPeriod[1]
//! ```
//!
//! The output `dc_period` is `SmoothPeriod` — a smoothed estimate of the
//! dominant cycle period in bars.


use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::hilberttransform::ht_kernel;
use crate::indicators::simd_indicators::hilberttransform_simd::ht_kernel_base as ht_kernel_q_simd;
use crate::ring_buffer::fixed_single_buffer::FixedRingBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::homodynediscriminator_simd::indicator_by_assets;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::homodynediscriminator_simd::indicator_by_assets as indicator;
}
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
/// Zero — all constants are fixed by Ehlers.
pub const OPTIONS_WIDTH: usize = 0;

/// Metadata describing the Homodyne Discriminator indicator.
pub const INFO: Info = Info {
    name: "homodynediscriminator",
    indicator_type: IndicatorType::Cycle,
    full_name: "Ehlers Homodyne Discriminator",
    inputs: &["real"],
    options: &[],
    outputs: &["dc_period"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "homodynediscriminator",
        label: "Ehlers Homodyne Discriminator",
        display_type: DisplayType::Indicator,
        outputs: &["dc_period"],
    }],
};

/// Per-bar state for the Ehlers Homodyne Discriminator.
///
/// Cascades four applications of the Hilbert Transform kernel through four ring
/// buffers, then uses the homodyne principle (dot/cross product of the phasor
/// with its 1-bar-delayed conjugate) to extract the instantaneous phase change
/// per bar, from which the dominant cycle `SmoothPeriod` is derived.
///
/// All fields initialise to `0.0` / empty. The IIR scalars and period state
/// remain at `0.0` throughout the `init_state` warmup (the early-return guards in
/// [`calc`] prevent them from being touched until all five buffers are full).
/// This matches EasyLanguage's implicit zero-initialisation.
#[derive(Serialize, Deserialize, Clone)]
pub struct State {
    // ── Stage 0: 4-bar Hann-weighted smooth ──────────────────────────────────
    pub price_buf: FixedRingBuffer<f64, 4>,

    // ── Stage 1: smooth → Detrender ──────────────────────────────────────────
    pub smooth_buf: FixedRingBuffer<f64, 7>,

    // ── Stage 2: Detrender → I1, Q1 ──────────────────────────────────────────
    pub detrender_buf: FixedRingBuffer<f64, 7>,

    // ── Stage 3: [I1, Q1] pairs as Simd<f64, 2> → jI / jQ (90° phase advance)
    // Lane 0 = i1, lane 1 = q1. One push and one set of index lookups serve
    // both kernels simultaneously via 128-bit SIMD loads and FMAs.
    pub iq1_buf: FixedRingBuffer<Simd<f64, 2>, 7>,

    // ── IIR scalar state for the homodyne discriminator ───────────────────────
    pub i2_prev: f64,
    pub q2_prev: f64,
    pub re_prev: f64,
    pub im_prev: f64,

    // ── Period tracking ───────────────────────────────────────────────────────
    /// Clamped and first-smoothed period (0.2/0.8 IIR). Used as the adaptive
    /// gain denominator on the next bar and to compute `smooth_period`.
    pub period: f64,
    /// Dominant cycle period estimate — the primary output (`SmoothPeriod`).
    pub smooth_period: f64,
}

impl State {
    /// Creates a new, zeroed state ready for the first bar.
    pub fn new() -> Self {
        Self {
            price_buf: FixedRingBuffer::new(),
            smooth_buf: FixedRingBuffer::new(),
            detrender_buf: FixedRingBuffer::new(),
            iq1_buf: FixedRingBuffer::new(),
            i2_prev: 0.0,
            q2_prev: 0.0,
            re_prev: 0.0,
            im_prev: 0.0,
            period: 0.0,
            smooth_period: 0.0,
        }
    }

    /// Returns `true` once all five ring buffers are full and `calc_unchecked`
    /// is safe to call. `i1_buf` and `q1_buf` are the last to fill — checking
    /// either one is sufficient since they are pushed in lockstep.
    #[inline(always)]
    pub(crate) fn all_buffers_full(&self) -> bool {
        self.iq1_buf.is_full()
    }

    /// Builds a warmed-up state by processing bars from `real` one at a time
    /// until all five ring buffers are full, then returns.
    ///
    /// This mirrors the `while buffer.len() < buffer.capacity()` pattern used in
    /// `HilbertTransform::init_state`: the loop condition is the proof of fullness,
    /// so `calc_unchecked` is always safe on entry to `cycle`.
    pub fn init_state(real: &[f64]) -> Self {
        let mut state = Self::new();
        let mut i = 0;
        while !state.all_buffers_full() {
            state.calc(real[i]);
            i += 1;
        }
        state
    }

    /// One-bar update. Returns `dc_period` (SmoothPeriod).
    ///
    /// Returns `0.0` while any ring buffer is still filling (warmup guard).
    /// All buffers are guaranteed full after [`init_state`], so the guards are
    /// cost-free on every production bar.
    #[inline(always)]
    pub fn calc(&mut self, real: f64) -> f64 {
        // ── Stage 0: 4-bar Hann smooth ───────────────────────────────────────
        // Two independent FMAs (ab, cd) reduce serial depth from 4 to 2;
        // * 0.1 avoids the more expensive / 10.0 division.
        self.price_buf.push(real);
        if self.price_buf.len() < 4 {
            return 0.0;
        }
        let ab = 4.0_f64.mul_add(self.price_buf[0], 3.0 * self.price_buf[1]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * 0.1;

        // ── Adaptive gain from the previous period estimate ───────────────────
        let gain = 0.075_f64.mul_add(self.period, 0.54);

        // ── Stage 1: Detrender ────────────────────────────────────────────────
        self.smooth_buf.push(smooth);
        if self.smooth_buf.len() < 7 {
            return 0.0;
        }
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        // ── Stage 2: I1, Q1 ──────────────────────────────────────────────────
        self.detrender_buf.push(detrender);
        if self.detrender_buf.len() < 7 {
            return 0.0;
        }
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        // ── Stage 3: jI and jQ via a single Simd<f64, 2> kernel pass ─────────
        // One push, 4 period_to_idx calls, four 128-bit loads, 4 SIMD FMAs
        // replace two scalar kernel calls (2 pushes, 8 index calls, 8 loads, 8 FMAs).
        self.iq1_buf.push(Simd::<f64, 2>::from_array([i1, q1]));
        if self.iq1_buf.len() < 7 {
            return 0.0;
        }
        let [j_i, j_q] = ht_kernel_q_simd(&self.iq1_buf, Simd::<f64, 2>::splat(gain)).to_array();

        self.apply_discriminator(i1, q1, j_i, j_q)
    }

    /// One-bar update returning `(smooth_period, i1, q1)`.
    ///
    /// Extends [`calc`](Self::calc) with the intermediate I1 / Q1 values from stage 2
    /// of the Hilbert pipeline. These are needed by callers (e.g. MAMA) that derive
    /// additional quantities (phase, adaptive alpha) from the same pipeline run.
    ///
    /// Returns `(0.0, 0.0, 0.0)` while any ring buffer is still filling (warmup guard).
    #[inline(always)]
    pub fn calc_with_iq(&mut self, real: f64) -> (f64, f64, f64) {
        self.price_buf.push(real);
        if self.price_buf.len() < 4 {
            return (0.0, 0.0, 0.0);
        }
        let ab = 4.0_f64.mul_add(self.price_buf[0], 3.0 * self.price_buf[1]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * 0.1;

        let gain = 0.075_f64.mul_add(self.period, 0.54);

        self.smooth_buf.push(smooth);
        if self.smooth_buf.len() < 7 {
            return (0.0, 0.0, 0.0);
        }
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        self.detrender_buf.push(detrender);
        if self.detrender_buf.len() < 7 {
            return (0.0, 0.0, 0.0);
        }
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        self.iq1_buf.push(Simd::<f64, 2>::from_array([i1, q1]));
        if self.iq1_buf.len() < 7 {
            return (0.0, 0.0, 0.0);
        }
        let [j_i, j_q] = ht_kernel_q_simd(&self.iq1_buf, Simd::<f64, 2>::splat(gain)).to_array();

        let dc_period = self.apply_discriminator(i1, q1, j_i, j_q);
        (dc_period, i1, q1)
    }

    /// Unsafe one-bar update returning `(smooth_period, i1, q1)` — skips all
    /// ring-buffer fullness guards.
    ///
    /// # Safety
    ///
    /// All five ring buffers must be full on entry. Guaranteed after [`init_state`].
    #[inline(always)]
    pub unsafe fn calc_unchecked_with_iq(&mut self, real: f64) -> (f64, f64, f64) {
        self.price_buf.push_unchecked(real);
        let ab = 4.0_f64.mul_add(self.price_buf[0], 3.0 * self.price_buf[1]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * 0.1;

        let gain = 0.075_f64.mul_add(self.period, 0.54);

        self.smooth_buf.push_unchecked(smooth);
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        self.detrender_buf.push_unchecked(detrender);
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        self.iq1_buf
            .push_unchecked(Simd::<f64, 2>::from_array([i1, q1]));
        let [j_i, j_q] = ht_kernel_q_simd(&self.iq1_buf, Simd::<f64, 2>::splat(gain)).to_array();

        let dc_period = self.apply_discriminator(i1, q1, j_i, j_q);
        (dc_period, i1, q1)
    }

    /// Unsafe one-bar update — skips all ring-buffer fullness guards
    /// `push_unchecked` on every buffer.
    ///
    /// # Safety
    ///
    /// All five ring buffers must be full on entry. This is guaranteed after
    /// [`init_state`] and on every subsequent bar in the cycle loop.
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, real: f64) -> f64 {
        // ── Stage 0: 4-bar Hann smooth ───────────────────────────────────────
        self.price_buf.push_unchecked(real);
        let ab = 4.0_f64.mul_add(self.price_buf[0], 3.0 * self.price_buf[1]);
        let cd = 2.0_f64.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * 0.1;

        let gain = 0.075_f64.mul_add(self.period, 0.54);

        // ── Stage 1: Detrender ────────────────────────────────────────────────
        self.smooth_buf.push_unchecked(smooth);
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        // ── Stage 2: I1, Q1 ──────────────────────────────────────────────────
        self.detrender_buf.push_unchecked(detrender);
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        // ── Stage 3: jI and jQ via a single Simd<f64, 2> kernel pass ─────────
        self.iq1_buf
            .push_unchecked(Simd::<f64, 2>::from_array([i1, q1]));
        let [j_i, j_q] = ht_kernel_q_simd(&self.iq1_buf, Simd::<f64, 2>::splat(gain)).to_array();

        self.apply_discriminator(i1, q1, j_i, j_q)
    }

    /// Applies the homodyne discriminator and period-smoothing logic.
    ///
    /// Shared by both [`calc`](Self::calc) and [`calc_unchecked`](Self::calc_unchecked).
    /// Mutates all IIR scalar state and returns the updated `SmoothPeriod`.
    #[inline(always)]
    fn apply_discriminator(&mut self, i1: f64, q1: f64, j_i: f64, j_q: f64) -> f64 {
        // ── Phasor rotation ───────────────────────────────────────────────────
        let i2_raw = i1 - j_q;
        let q2_raw = q1 + j_i;

        // ── IIR smooth I2, Q2 (α = 0.2) — two independent FMAs ───────────────
        let i2 = 0.2_f64.mul_add(i2_raw, 0.8 * self.i2_prev);
        let q2 = 0.2_f64.mul_add(q2_raw, 0.8 * self.q2_prev);

        // ── Homodyne discriminator: dot / cross with 1-bar-delayed conjugate ──
        // Re = I2·I2[1] + Q2·Q2[1]  (dot product → cosine of phase difference)
        // Im = I2·Q2[1] − Q2·I2[1]  (cross product → sine of phase difference)
        let re_raw = i2.mul_add(self.i2_prev, q2 * self.q2_prev);
        let im_raw = i2.mul_add(self.q2_prev, -(q2 * self.i2_prev));

        self.i2_prev = i2;
        self.q2_prev = q2;

        // ── IIR smooth Re, Im (α = 0.2) ───────────────────────────────────────
        let re = 0.2_f64.mul_add(re_raw, 0.8 * self.re_prev);
        let im = 0.2_f64.mul_add(im_raw, 0.8 * self.im_prev);

        self.re_prev = re;
        self.im_prev = im;

        // ── Period from instantaneous phase-change per bar ────────────────────
        let mut period = if im != 0.0 && re != 0.0 {
            std::f64::consts::TAU / (im / re).atan()
        } else {
            self.period
        };

        // ── Rate-of-change clamp: ±50% per bar ───────────────────────────────
        period = period.min(1.5 * self.period);
        period = period.max(0.67 * self.period);

        // ── Absolute clamp [6, 50] bars ───────────────────────────────────────
        period = period.clamp(6.0, 50.0);

        // ── Final smoothing ───────────────────────────────────────────────────
        period = 0.2_f64.mul_add(period, 0.8 * self.period);
        self.smooth_period = 0.33_f64.mul_add(period, 0.67 * self.smooth_period);
        self.period = period;

        self.smooth_period
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming indicator state, wrapping [`State`] for use with [`batch_indicator`].
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
}

impl IndicatorState {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let len = inputs[0].len();
        let mut dc_period_line = crate::uninit_vec!(f64, len);
        cycle(inputs[0], &mut self.state, &mut dc_period_line);
        Ok(vec![dc_period_line])
    }
}

/// Returns the minimum number of input bars required for the Homodyne Discriminator.
///
/// Fixed at 23: the 4-bar Hann smooth requires 4 bars, then each of the three
/// Hilbert kernel stages needs 6 additional bars to fill its 7-slot ring buffer,
/// giving bar 21 (0-indexed) as the first bar where all five ring buffers become
/// simultaneously full. `init_state` processes those 22 bars (0..21 inclusive),
/// leaving all buffers fully counted so that `calc_unchecked` is safe from bar 22
/// onwards. Bar 22 is therefore the first bar returned in the output.
pub fn min_data(_options: &[f64]) -> usize {
    23
}

/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// The Homodyne Discriminator uses fixed-coefficient IIR filters whose warmup
/// is independent of decimal precision, so this always delegates to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Returns the number of output values produced for a given input length.
///
/// # Arguments
///
/// * `data_len` - Length of the input price series.
/// * `options` - Unused; pass `&[]`.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Ehlers Homodyne Discriminator over the full input dataset.
///
/// Applies the 4-bar Hann smooth, then cascades three Hilbert kernel stages
/// with adaptive gain `0.075·Period[1] + 0.54` to produce in-phase and quadrature
/// components. The homodyne principle (dot/cross product of the phasor with its
/// 1-bar-delayed conjugate) extracts the instantaneous phase change per bar,
/// from which the dominant cycle period is derived via `2π / atan(Im / Re)`,
/// clamped to `[6, 50]`, and smoothed to produce `SmoothPeriod`.
///
/// # Inputs
///
/// * `inputs[0]` — `real` price series (typically close, or `(high + low) / 2`).
///
/// # Options
///
/// None. All constants are fixed by Ehlers. Pass `&[]`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `dc_period` (SmoothPeriod) with
/// length `output_length(data_len, options)`.
/// Returns `Err(IndicatorError::NotEnoughData)` if the input is shorter than
/// [`min_data`].
///
/// > **Note:** The IIR smoothers (I2, Q2, Re, Im, Period, SmoothPeriod) all start
/// > from 0.0 and require additional bars beyond the ring-buffer warmup to converge.
/// > Treat the first ~50 output bars as transient.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(options))?;
    let capacity = output_length(inputs[0].len(), options);
    let mut dc_period_line = crate::uninit_vec!(f64, capacity);

    let mut state = State::init_state(inputs[0]);
    // Slice starting at bar (min_data - 1) = 21, the first bar with valid output
    let real = &inputs[0][(min_data(options) - 1)..];
    cycle(real, &mut state, &mut dc_period_line);

    Ok((vec![dc_period_line], IndicatorState::new(state)))
}

/// Core calculation loop for the Homodyne Discriminator.
///
/// # Arguments
///
/// * `real` - Input slice starting at bar `min_data - 1` (already offset by the caller).
/// * `state` - Mutable state; all five ring buffers must be full on entry.
/// * `dc_period_line` - Output slice; must be the same length as `real`.
fn cycle(real: &[f64], state: &mut State, dc_period_line: &mut [f64]) {
    for i in 0..real.len() {
        let dc = unsafe { state.calc_unchecked(*real.get_unchecked(i)) };
        unsafe {
            *dc_period_line.get_unchecked_mut(i) = dc;
        }
    }
}
