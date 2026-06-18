//! # MESA Adaptive Moving Average (MAMA / FAMA)
//!
//! **Source:** John Ehlers, *Rocket Science for Traders* (2001), Chapter 8.
//! Originally published as "MESA Adaptive Moving Averages",
//! *Technical Analysis of Stocks & Commodities*, February 2001.
//!
//! An exponential moving average whose smoothing factor α adapts bar-by-bar
//! based on how fast the instantaneous phase of the dominant market cycle is
//! changing. Slow phase change → large α (responsive); fast phase change →
//! small α (more smoothing). FAMA (Following Adaptive Moving Average) uses
//! half the α of MAMA to create a lagging counterpart for signal generation.
//!
//! ## Formula
//!
//! The indicator builds on the [`homodynediscriminator`] (HD) pipeline.
//! Stages 0–3 of the HD produce I1 and Q1, from which:
//!
//! ```text
//! Phase      = atan(Q1 / I1) × (180 / π)        degrees; 0 if I1 = 0
//! DeltaPhase = max(Phase[1] − Phase, 1.0)        1° floor prevents ÷0
//! α          = clamp(FastLimit / DeltaPhase, SlowLimit, FastLimit)
//!
//! MAMA = α · Price + (1 − α) · MAMA[1]
//! FAMA = 0.5α · MAMA + (1 − 0.5α) · FAMA[1]
//! ```
//!
//! Phase is expressed in degrees to match Ehlers' EasyLanguage `Atan` convention
//! and TA-Lib's implementation (both use degrees), ensuring that
//! `FastLimit / DeltaPhase` yields the expected alpha range for Ehlers' default
//! parameters (FastLimit = 0.5, SlowLimit = 0.05).
//!
//! ## TA-Lib `MAMA`
//!
//! TA-Lib's `MAMA` implements the same Ehlers 2001 formula. Outputs are close
//! but not bit-identical due to the different HD kernel implementation (TA-Lib
//! uses alternating even/odd buffers; we use a 7-slot ring buffer). The
//! benchmark comparison is valid as a throughput measurement.

use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::homodynediscriminator;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::mama_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::mama_simd::indicator_by_options;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::mama_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::mama_simd::indicator_by_options as indicator;
}

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
/// `[fast_limit, slow_limit]`
pub const OPTIONS_WIDTH: usize = 2;

/// Metadata describing the MAMA / FAMA indicator.
pub const INFO: Info = Info {
    name: "mama",
    indicator_type: IndicatorType::Trend,
    full_name: "MESA Adaptive Moving Average",
    inputs: &["real"],
    options: &["fast_limit", "slow_limit"],
    outputs: &["mama", "fama"],
    optional_outputs: &["dc_period", "alpha"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "mama",
            label: "MAMA / FAMA",
            display_type: DisplayType::Overlay,
            outputs: &["mama", "fama"],
        },
        DisplayGroup {
            offset: None,
            id: "mama_dc_period",
            label: "MAMA Dominant Cycle Period",
            display_type: DisplayType::Indicator,
            outputs: &["dc_period"],
        },
        DisplayGroup {
            offset: None,
            id: "mama_alpha",
            label: "MAMA Alpha",
            display_type: DisplayType::Indicator,
            outputs: &["alpha"],
        },
    ],
};

/// Per-bar state for the Ehlers MESA Adaptive Moving Average (MAMA) and
/// Following Adaptive Moving Average (FAMA).
///
/// Composes the full [`homodynediscriminator::State`] pipeline (4-bar Hann smooth →
/// Detrender → I1/Q1 → jI/jQ → homodyne discriminator) and extends it with the
/// MAMA-specific stage:
///
/// ```text
/// Phase      = atan(Q1 / I1) × (180 / π)          // degrees — matches Ehlers & TA-Lib
/// DeltaPhase = max(Phase[1] − Phase, 1.0)           // 1° floor prevents ÷0
/// α          = clamp(FastLimit / DeltaPhase, SlowLimit, FastLimit)
/// MAMA       = α · Price + (1 − α) · MAMA[1]
/// FAMA       = ½α · MAMA  + (1 − ½α) · FAMA[1]
/// ```
///
/// Phase is expressed in degrees so that `fast_limit / delta_phase` produces the
/// expected alpha range with Ehlers' defaults (FastLimit = 0.5, SlowLimit = 0.05),
/// matching both the EasyLanguage original and TA-Lib's `TA_MAMA`.
///
/// **Warmup:** The first valid output is emitted at bar `min_data - 1 = 22` (0-indexed),
/// which is the same warmup as the Homodyne Discriminator. On that bar `mama = fama = price`
/// exactly (seeded from the first-output-bar price so that α·p + (1−α)·p = p). Subsequent
/// bars evolve from this seed.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// Full Homodyne Discriminator pipeline (stages 0–3 and IIR discriminator).
    pub hd: homodynediscriminator::State,

    /// Instantaneous phase (degrees) from the previous bar — used to compute DeltaPhase.
    pub prev_phase: f64,

    /// Current MAMA value.
    pub mama: f64,

    /// Current FAMA value.
    pub fama: f64,

    /// Last computed adaptive alpha (stored for optional output; cheap to keep vs. re-compute).
    pub alpha: f64,
}

impl State {
    /// Creates a new, zeroed state ready for the first bar.
    pub fn new() -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            prev_phase: 0.0,
            mama: 0.0,
            fama: 0.0,
            alpha: 0.0,
        }
    }

    /// Builds a warmed-up state by feeding bars one at a time until all HD ring
    /// buffers are full, then processes the first valid bar (bar `min_data − 1`)
    /// with MAMA seeded from that bar's price so that `mama[0] = fama[0] = price`.
    ///
    /// Writes the first output values to `mama_line[0]` and `fama_line[0]`, and
    /// optionally to `dc_period_line[0]` / `alpha_line[0]` if those slices are
    /// non-empty. The caller should pass empty slices for unneeded optional outputs.
    pub fn init_state(
        real: &[f64],
        fast_limit: f64,
        slow_limit: f64,
        mama_line: &mut [f64],
        fama_line: &mut [f64],
        dc_period_line: &mut [f64],
        alpha_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new();
        let mut i = 0;

        // Feed warmup bars through the HD pipeline only (no MAMA computation needed).
        while !state.hd.all_buffers_full() {
            state.hd.calc(real[i]);
            i += 1;
        }

        // After the loop, i = min_data - 1 = 22 (the first output bar, 0-indexed).
        // Seed MAMA and FAMA from that bar's price so the first output is price-exact:
        //   mama = α·price + (1−α)·price_seed = price (when price_seed = price).
        let seed = real[i];
        state.mama = seed;
        state.fama = seed;

        // Process bar 22 (first valid bar) — HD buffers are full, safe to use unchecked.
        let (mama, fama) = unsafe { state.calc_unchecked(real[i], fast_limit, slow_limit) };
        mama_line[0] = mama;
        fama_line[0] = fama;

        let (_, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);
        crate::store_optional_outputs!(0,
            want_dc,    dc_period_line => state.hd.smooth_period,
            want_alpha, alpha_line     => state.alpha
        );

        state
    }

    /// One-bar update. Returns `(mama, fama)`.
    ///
    /// Returns `(0.0, 0.0)` while any HD ring buffer is still filling (warmup guard).
    /// After [`init_state`] all buffers are guaranteed full, so the guards are
    /// cost-free on every production bar.
    #[inline(always)]
    pub fn calc(&mut self, real: f64, fast_limit: f64, slow_limit: f64) -> (f64, f64) {
        let (_, i1, q1) = self.hd.calc_with_iq(real);
        if !self.hd.all_buffers_full() {
            return (0.0, 0.0);
        }
        self.apply_mama(real, i1, q1, fast_limit, slow_limit);
        (self.mama, self.fama)
    }

    /// Unsafe one-bar update — skips all ring-buffer fullness guards.
    ///
    /// # Safety
    ///
    /// All HD ring buffers must be full on entry. This is guaranteed after
    /// [`init_state`] and on every subsequent bar in the cycle loop.
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        real: f64,
        fast_limit: f64,
        slow_limit: f64,
    ) -> (f64, f64) {
        let (_, i1, q1) = self.hd.calc_unchecked_with_iq(real);
        self.apply_mama(real, i1, q1, fast_limit, slow_limit);
        (self.mama, self.fama)
    }

    /// Applies the MAMA-specific computation: phase delta → adaptive alpha → EMA updates.
    ///
    /// Shared by [`calc`](Self::calc) and [`calc_unchecked`](Self::calc_unchecked).
    /// Updates `prev_phase`, `alpha`, `mama`, and `fama` in place.
    ///
    /// Phase is converted to degrees (`× 180/π`) to match Ehlers' EasyLanguage `Atan`
    /// convention and TA-Lib's implementation, ensuring that `fast_limit / delta_phase`
    /// produces the expected alpha range with Ehlers' default parameters.
    #[inline(always)]
    fn apply_mama(&mut self, real: f64, i1: f64, q1: f64, fast_limit: f64, slow_limit: f64) {
        const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

        // Instantaneous phase in degrees. Guard against I1 = 0 (undefined atan).
        let phase = if i1 != 0.0 {
            (q1 / i1).atan() * RAD_TO_DEG
        } else {
            0.0
        };

        // Phase decreases (advances) as cycles progress, so DeltaPhase = prev − current.
        // Floor at 1° to prevent division by zero or absurdly large alpha.
        let delta_phase = (self.prev_phase - phase).max(1.0);
        self.prev_phase = phase;

        // Adaptive alpha: larger when phase barely moved (slow market), capped at FastLimit.
        self.alpha = (fast_limit / delta_phase).clamp(slow_limit, fast_limit);

        // MAMA — standard EMA with adaptive alpha.
        self.mama = self.alpha.mul_add(real, (1.0 - self.alpha) * self.mama);

        // FAMA — slower EMA at half the alpha, tracking MAMA.
        let half_alpha = 0.5 * self.alpha;
        self.fama = half_alpha.mul_add(self.mama, (1.0 - half_alpha) * self.fama);
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming indicator state, wrapping [`State`] together with the fixed limits
/// for use with [`batch_indicator`].
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    fast_limit: f64,
    slow_limit: f64,
}

impl IndicatorState {
    pub fn new(state: State, fast_limit: f64, slow_limit: f64) -> Self {
        Self {
            state,
            fast_limit,
            slow_limit,
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
        let len = inputs[0].len();

        let (mut mama_line, mut fama_line, (mut dc_period_line, mut alpha_line)) = (
            crate::uninit_vec!(f64, len),
            crate::uninit_vec!(f64, len),
            crate::init_optional_outputs!(
                optional_outputs, &[false, false],
                dc_period_line: len,
                alpha_line: len
            ),
        );

        cycle(
            inputs[0],
            &mut self.state,
            self.fast_limit,
            self.slow_limit,
            &mut mama_line,
            &mut fama_line,
            (&mut dc_period_line, &mut alpha_line),
        );

        Ok(vec![mama_line, fama_line, dc_period_line, alpha_line])
    }
}

/// Returns the minimum number of input bars required for MAMA / FAMA.
///
/// Fixed at 23 — identical to the Homodyne Discriminator warmup, since MAMA
/// embeds the full HD pipeline and adds no extra ring buffers.
pub fn min_data(_options: &[f64]) -> usize {
    23
}

/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// MAMA uses fixed-coefficient IIR filters whose warmup is independent of
/// decimal precision, so this always delegates to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Returns the number of output values produced for a given input length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Validates MAMA options.
///
/// # Errors
///
/// Returns [`IndicatorError::InvalidOptions`] if:
/// - `fast_limit` is not in `(0.0, 1.0]`
/// - `slow_limit` is not in `(0.0, fast_limit)`
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    let fast = options[0];
    let slow = options[1];
    if fast <= 0.0 || fast > 1.0 || slow <= 0.0 || slow >= fast {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Calculates MAMA and FAMA over the full input dataset.
///
/// Applies the Ehlers Homodyne Discriminator pipeline to extract the dominant
/// cycle period, then uses the instantaneous phase rate of change to derive
/// an adaptive smoothing coefficient α for each bar:
///
/// ```text
/// α = clamp(fast_limit / DeltaPhase°, slow_limit, fast_limit)
/// MAMA = α · price + (1 − α) · MAMA[1]
/// FAMA = ½α · MAMA  + (1 − ½α) · FAMA[1]
/// ```
///
/// # Inputs
///
/// * `inputs[0]` — `real` price series (typically close, or `(high + low) / 2`)
///
/// # Options
///
/// * `options[0]` — `fast_limit`: maximum alpha (Ehlers default: **0.5**)
/// * `options[1]` — `slow_limit`: minimum alpha (Ehlers default: **0.05**)
///
/// # Optional outputs
///
/// * index 0 — `dc_period`: dominant cycle period (`SmoothPeriod` from the embedded HD)
/// * index 1 — `alpha`: adaptive smoothing coefficient used each bar
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `mama`, `outputs[1]` is `fama`,
/// `outputs[2]` is `dc_period` (empty unless requested), and `outputs[3]` is
/// `alpha` (empty unless requested). `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming updates.
///
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
///
/// > **Note:** The first ~50 output bars should be treated as transient while the
/// > embedded IIR smoothers (I2, Q2, Re, Im, Period) converge from their zero seeds.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let fast_limit = options[0];
    let slow_limit = options[1];

    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];
    let capacity = output_length(real.len(), options);

    let (mut mama_line, mut fama_line, (mut dc_period_line, mut alpha_line)) = (
        crate::uninit_vec!(f64, capacity),
        crate::uninit_vec!(f64, capacity),
        crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            dc_period_line: capacity,
            alpha_line: capacity
        ),
    );

    // init_state fills HD buffers, seeds MAMA/FAMA from first-output-bar price,
    // processes bar (min_data − 1) = 22, and writes to output[0].
    let mut state = State::init_state(
        real,
        fast_limit,
        slow_limit,
        &mut mama_line,
        &mut fama_line,
        &mut dc_period_line,
        &mut alpha_line,
    );

    // cycle processes bars min_data..len-1 and writes to output[1..].
    let real_tail = &real[min_data(options)..];
    let (_, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);
    let dc_tail = if want_dc {
        &mut dc_period_line[1..]
    } else {
        &mut dc_period_line[..]
    };
    let alpha_tail = if want_alpha {
        &mut alpha_line[1..]
    } else {
        &mut alpha_line[..]
    };

    cycle(
        real_tail,
        &mut state,
        fast_limit,
        slow_limit,
        &mut mama_line[1..],
        &mut fama_line[1..],
        (dc_tail, alpha_tail),
    );

    Ok((
        vec![mama_line, fama_line, dc_period_line, alpha_line],
        IndicatorState::new(state, fast_limit, slow_limit),
    ))
}

/// Core calculation loop for MAMA / FAMA.
///
/// All HD ring buffers must be full on entry (guaranteed after `init_state`).
/// Writes `mama` and `fama` to the corresponding output slices, and optionally
/// `dc_period` / `alpha` when those slices are non-empty.
fn cycle(
    real: &[f64],
    state: &mut State,
    fast_limit: f64,
    slow_limit: f64,
    mama_line: &mut [f64],
    fama_line: &mut [f64],
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (dc_period_line, alpha_line) = optional_outputs;
    let (has_optional, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);

    for i in 0..real.len() {
        let (mama, fama) =
            unsafe { state.calc_unchecked(*real.get_unchecked(i), fast_limit, slow_limit) };

        unsafe {
            *mama_line.get_unchecked_mut(i) = mama;
            *fama_line.get_unchecked_mut(i) = fama;
        }

        if has_optional {
            crate::store_optional_outputs!(i,
                want_dc,    dc_period_line => state.hd.smooth_period,
                want_alpha, alpha_line     => state.alpha
            );
        }
    }
}
